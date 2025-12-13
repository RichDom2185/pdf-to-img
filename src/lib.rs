mod opts;

use std::io::prelude::*;
use wasm_bindgen::prelude::*;

use hayro::{InterpreterSettings, Pdf, RenderSettings, render};
use image::ImageReader;
use once_cell::sync::Lazy;
use std::{io::Cursor, sync::Arc};
use web_sys::{
    Blob, BlobPropertyBag,
    js_sys::{self, Array},
};
use web_time::Instant;
use zip::{ZipWriter, write::SimpleFileOptions};

static INTERPRETER_SETTINGS: Lazy<InterpreterSettings> = Lazy::new(InterpreterSettings::default);
static RENDER_SETTINGS: Lazy<RenderSettings> = Lazy::new(RenderSettings::default);
static ZIP_FILE_OPTIONS: Lazy<SimpleFileOptions> =
    Lazy::new(|| SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored));

// Options type for thumbnail extraction
pub struct ThumbnailOptions {
    pub width: u32,
    pub height: u32,
}

fn log(args: impl std::fmt::Display) {
    web_sys::console::log_1(&args.to_string().into());
}

fn log_error(args: impl std::fmt::Display) {
    web_sys::console::error_1(&args.to_string().into());
}

fn convert_png_to_jpeg(png_bytes: &[u8]) -> Vec<u8> {
    let rgba_reader = ImageReader::with_format(Cursor::new(png_bytes), image::ImageFormat::Png)
        .decode()
        .unwrap();
    let mut jpeg_bytes: Vec<u8> = Vec::new();
    rgba_reader
        .write_to(&mut Cursor::new(&mut jpeg_bytes), image::ImageFormat::Jpeg)
        .unwrap();
    jpeg_bytes
}

fn pixmap_to_bytes(pixmap: hayro::Pixmap, format: &opts::ThumbnailFormat) -> Vec<u8> {
    match format {
        opts::ThumbnailFormat::Png => pixmap.take_png(),
        opts::ThumbnailFormat::Jpeg => {
            let png_bytes = pixmap.take_png();
            convert_png_to_jpeg(&png_bytes)
        }
    }
}

fn bytes_to_blob(data: Vec<u8>, mime_type: &str) -> Blob {
    let array = Array::new();
    array.push(&js_sys::Uint8Array::from(&data[..]));
    let blob_opts = BlobPropertyBag::new();
    blob_opts.set_type(mime_type);
    Blob::new_with_u8_array_sequence_and_options(&array.into(), &blob_opts)
        .expect("failed to create blob from byte array")
}

#[wasm_bindgen]
pub fn convert_pdf(data: &[u8], raw_opts: JsValue) -> Blob {
    match convert_pdf_internal(data, raw_opts) {
        Ok(data) => bytes_to_blob(data, "application/zip"),
        Err(err_msg) => {
            log_error(format!("Error during PDF conversion: {}", err_msg));
            bytes_to_blob(Vec::new(), "application/zip")
        }
    }
}

fn convert_pdf_internal(data: &[u8], raw_opts: JsValue) -> Result<Vec<u8>, &'static str> {
    log("Starting thumbnail extraction...");
    let opts: opts::ThumbnailOptions = serde_wasm_bindgen::from_value(raw_opts)
        .map_err(|_| "failed to parse options for thumbnail extraction")?;
    log(format!("Thumbnail options:format={:?}", opts.format));
    let pdf = Pdf::new(Arc::new(data.to_vec())).map_err(|_| "failed reading document")?;
    log(format!("PDF has {} pages", pdf.pages().len()));

    let now = Instant::now();
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    for (page_num, page) in pdf.pages().iter().enumerate() {
        let page_num = page_num + 1;
        let page_pixmap = render(page, &INTERPRETER_SETTINGS, &RENDER_SETTINGS);
        log(format!(
            "Generating {:?} thumbnail for page {}",
            opts.format, page_num
        ));
        let image_bytes = pixmap_to_bytes(page_pixmap, &opts.format);
        let filename = format!(
            "page-{:0>3}.{}",
            page_num,
            match opts.format {
                opts::ThumbnailFormat::Png => "png",
                opts::ThumbnailFormat::Jpeg => "jpeg",
            }
        );
        zip.start_file(filename, *ZIP_FILE_OPTIONS)
            .map_err(|_| "failed to start file in archive")?;
        zip.write_all(&image_bytes)
            .map_err(|_| "failed to write image data to archive")?;

        log(format!(
            "processed page {} in {:.2}s",
            page_num,
            now.elapsed().as_secs_f32()
        ));
    }
    log(format!(
        "Completed thumbnail extraction in {:.2}s",
        now.elapsed().as_secs_f32()
    ));
    let zip_data = zip
        .finish()
        .map_err(|_| "failed to finalize archive")?
        .into_inner();
    Ok(zip_data)
}
