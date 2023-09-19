use actix_files::NamedFile;
use actix_web::HttpRequest;
use actix_web::{get, web, App, HttpServer};
use image::io::Reader as ImageReader;
use image::{DynamicImage, ImageError};
use serde::Deserialize;
use std::fs;
use std::str;
use std::{env, io};

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Subject {
    Cage,
    Murray,
    Segall,
}

impl std::fmt::Display for Subject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Subject::Cage => write!(f, "cage"),
            Subject::Murray => write!(f, "murray"),
            Subject::Segall => write!(f, "segall"),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum ImageKind {
    Default,
    Crazy,
    Gif,
}

impl std::fmt::Display for ImageKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageKind::Crazy => write!(f, "crazy"),
            ImageKind::Default => write!(f, "default"),
            ImageKind::Gif => write!(f, "gif"),
        }
    }
}

fn get_image_count(subject: Subject, kind: ImageKind) -> u32 {
    match (subject, kind) {
        (Subject::Cage, ImageKind::Crazy) => 23,
        (Subject::Cage, ImageKind::Default) => 33,
        (Subject::Cage, ImageKind::Gif) => 43,
        (Subject::Murray, ImageKind::Default) => 23,
        (Subject::Segall, ImageKind::Default) => 30,
        _ => 0,
    }
}

fn resize_to_fill(
    width: u32,
    height: u32,
    input_path: &str,
    output_path: &str,
) -> Result<DynamicImage, ImageError> {
    let image = ImageReader::open(input_path)?.decode()?;
    let filter = if (width + height) > 3000 {
        image::imageops::FilterType::Nearest
    } else {
        image::imageops::FilterType::CatmullRom
    };
    let image_resized = image.resize_to_fill(width, height, filter);
    image_resized.save(output_path)?;
    Ok(image_resized)
}

fn resize_to_fill_io(
    width: u32,
    height: u32,
    input_path: &str,
    output_path: &str,
) -> io::Result<DynamicImage> {
    match resize_to_fill(width, height, input_path, output_path) {
        Err(image_error) => match image_error {
            ImageError::Decoding(e) => return Err(io::Error::new(io::ErrorKind::InvalidData, e)),
            ImageError::Encoding(e) => return Err(io::Error::new(io::ErrorKind::Unsupported, e)),
            ImageError::IoError(e) => return Err(io::Error::new(io::ErrorKind::Other, e)),
            ImageError::Limits(e) => return Err(io::Error::new(io::ErrorKind::Unsupported, e)),
            ImageError::Parameter(e) => return Err(io::Error::new(io::ErrorKind::InvalidInput, e)),
            ImageError::Unsupported(e) => {
                return Err(io::Error::new(io::ErrorKind::Unsupported, e))
            }
        },
        Ok(image) => return Ok(image),
    }
}

fn get_image(
    width: u32,
    height: u32,
    subject_op: Option<Subject>,
    kind_op: Option<ImageKind>,
) -> io::Result<String> {
    if width + height > 7000 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, ""));
    }

    let subject = subject_op.unwrap_or(Subject::Cage);
    let kind = kind_op.unwrap_or(ImageKind::Default);

    let current_dir = env::current_dir()?;
    let cur_path = current_dir.into_os_string().into_string().unwrap();
    let input_dir = format!("{cur_path}/images/source/{subject}/{kind}/");
    let output_dir = format!("{cur_path}/images/_gen/{subject}/{kind}/");
    fs::create_dir_all(&output_dir)?;

    let image_count = get_image_count(subject, kind);
    let selected_image = ((width + height) % image_count) + 1;
    let input_file = format!("{input_dir}/{selected_image}.jpg");
    let output_file = format!("{output_dir}/{width}x{height}.jpg");
    resize_to_fill_io(width, height, input_file.as_str(), output_file.as_str())?;

    return Ok(output_file);
}

#[derive(Deserialize)]
struct GetImageRequestInfo {
    subject: Subject,
    kind: ImageKind,
    width: u32,
    height: u32,
}

#[get("/{subject}/{kind}/{width}/{height}")]
async fn get_image_endpoint(
    _req: HttpRequest,
    path: web::Path<GetImageRequestInfo>,
) -> io::Result<NamedFile> {
    let GetImageRequestInfo {
        subject,
        kind,
        width,
        height,
    } = path.into_inner();

    let output_file = get_image(width, height, Some(subject), Some(kind))?;

    Ok(NamedFile::open_async(output_file).await?)
}

#[derive(Deserialize)]
struct GetNoKindImageRequestInfo {
    subject: Subject,
    width: u32,
    height: u32,
}

#[get("/{subject}/{width}/{height}")]
async fn get_no_kind_image_endpoint(
    _req: HttpRequest,
    path: web::Path<GetNoKindImageRequestInfo>,
) -> io::Result<NamedFile> {
    let GetNoKindImageRequestInfo {
        subject,
        width,
        height,
    } = path.into_inner();

    let output_file = get_image(width, height, Some(subject), None)?;

    Ok(NamedFile::open_async(output_file).await?)
}

#[derive(Deserialize)]
struct GetNoKindNoSubjectImageRequestInfo {
    width: u32,
    height: u32,
}

#[get("/{width}/{height}")]
async fn get_no_kind_no_subject_image_endpoint(
    _req: HttpRequest,
    path: web::Path<GetNoKindNoSubjectImageRequestInfo>,
) -> io::Result<NamedFile> {
    let GetNoKindNoSubjectImageRequestInfo { width, height } = path.into_inner();

    let output_file = get_image(width, height, None, None)?;

    Ok(NamedFile::open_async(output_file).await?)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(get_image_endpoint)
            .service(get_no_kind_image_endpoint)
            .service(get_no_kind_no_subject_image_endpoint)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
