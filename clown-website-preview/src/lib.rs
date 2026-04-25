use image::DynamicImage;
use std::io::{Cursor, Read};
use std::sync::Arc;

#[derive(Clone)]
pub struct MetaData {
    image_url: String,
    title: String,
    description: String,
    image: Option<Arc<DynamicImage>>,
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParserError {
    #[error("Cannot parse html")]
    ParseHTML(#[from] tl::ParseError),
}

impl MetaData {
    pub fn try_parse(text: &str) -> Result<Self, ParserError> {
        let dom = tl::parse(text, tl::ParserOptions::default())?;
        let parser = dom.parser();

        let mut meta = MetaData {
            image_url: String::new(),
            title: String::new(),
            description: String::new(),
            image: None,
        };

        if let Some(selector) = dom.query_selector("head") {
            if let Some(tag) = selector
                .into_iter()
                .next()
                .and_then(|v| v.get(parser).and_then(|node| node.as_tag()))
            {
                if let Some(selector) = tag.query_selector(parser, "meta") {
                    for handle in selector {
                        if let Some(tag) = handle.get(parser).and_then(|node| node.as_tag()) {
                            let attributes = tag.attributes();

                            let property = attributes
                                .get("property")
                                .flatten()
                                .map(|v| v.as_utf8_str());
                            let content =
                                attributes.get("content").flatten().map(|v| v.as_utf8_str());

                            if let (Some(prop), Some(cont)) = (property, content) {
                                match prop.as_ref() {
                                    "og:title" => meta.title = cont.to_string(),
                                    "og:image" => meta.image_url = cont.to_string(),
                                    "og:description" => meta.description = cont.to_string(),
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(meta)
    }

    pub fn get_title(&self) -> &str {
        &self.title
    }

    pub fn take_image(&mut self) -> Option<Arc<DynamicImage>> {
        self.image.take()
    }
}

fn parse_html(text: &str, is_meta: bool) -> Result<MetaData, ParserError> {
    if !is_meta {
        return Ok(MetaData {
            image_url: String::new(),
            title: String::new(),
            description: String::new(),
            image: None,
        });
    }

    MetaData::try_parse(text)
}

fn convert_bytes_to_image(bytes: Vec<u8>) -> Option<Arc<DynamicImage>> {
    if let Ok(image_reader) = image::ImageReader::new(Cursor::new(bytes)).with_guessed_format()
        && let Ok(image) = image_reader.decode()
    {
        Some(Arc::new(image))
    } else {
        None
    }
}

const MAX_BYTES: usize = 64 * 1024; // safety cap (64 KB)

fn fetch_head_html(reader: &mut dyn Read) -> Result<String, std::io::Error> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];

    loop {
        let n = reader.read(&mut chunk)?;
        if n == 0 {
            break;
        }

        let previous_size = buf.len();
        buf.extend_from_slice(&chunk[..n]);

        // Stop if </head> is found
        if let Some(b) = buf.get(previous_size.saturating_sub(7)..)
            && b.windows(7).any(|w| w.eq_ignore_ascii_case(b"</head>"))
        {
            break;
        }

        // Safety cap
        if buf.len() > MAX_BYTES {
            break;
        }
    }

    Ok(String::from_utf8_lossy(&buf).into_owned())
}

pub fn get_url_preview(endpoint: &str) -> Result<MetaData, String> {
    let resp = ureq::get(endpoint).call().map_err(|e| e.to_string())?;

    let content_type = resp
        .headers()
        .get("content-type")
        .map(|v| v.to_str().ok())
        .flatten();

    let has_meta = !content_type.is_some_and(|v| v.starts_with("image"));

    let metadata = if has_meta {
        let mut reader = resp.into_body().into_reader();
        let head = fetch_head_html(&mut reader).map_err(|e| e.to_string())?;
        let mut m = parse_html(&head, has_meta).map_err(|e| e.to_string())?;

        if !m.image_url.is_empty() {
            let img_resp = ureq::get(&m.image_url).call().map_err(|e| e.to_string())?;
            let mut img_bytes = Vec::new();
            img_resp
                .into_body()
                .into_reader()
                .read_to_end(&mut img_bytes)
                .map_err(|e| e.to_string())?;
            m.image = convert_bytes_to_image(img_bytes);
        }
        m
    } else {
        let mut bytes = Vec::new();
        resp.into_body()
            .into_reader()
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;

        MetaData {
            image_url: String::from(endpoint),
            title: String::from(""),
            description: String::from(""),
            image: convert_bytes_to_image(bytes),
        }
    };

    Ok(metadata)
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use std::sync::Arc;

    #[test]
    fn test_parse_meta_from_html() {
        let html = r#"
        <html>
            <head>
                <meta property="og:title" content="Example Title">
                <meta property="og:description" content="Example description.">
                <meta property="og:image" content="https://example.com/image.jpg">
                <meta property="og:site" content="ExampleSite">
            </head>
        </html>
        "#;

        let meta = MetaData::try_parse(html).unwrap();

        assert_eq!(meta.title, "Example Title");
        assert_eq!(meta.description, "Example description.");
        assert_eq!(meta.image_url, "https://example.com/image.jpg");
        assert!(meta.image.is_none());
    }

    #[test]
    fn test_parse_html_no_meta() {
        let html = "<html><body>No meta here</body></html>";
        let meta = parse_html(html, true).unwrap();
        assert_eq!(meta.title, "");
        assert_eq!(meta.image_url, "");
    }

    #[test]
    fn test_parse_html_disabled_meta_flag() {
        let html = "<html><head><meta property='og:title' content='Ignored'></head></html>";
        let meta = parse_html(html, false).unwrap();
        assert_eq!(meta.title, "");
    }

    #[test]
    fn test_take_image() {
        let mut meta = MetaData {
            image_url: String::from(""),
            title: String::from(""),
            description: String::from(""),
            image: Some(Arc::new(DynamicImage::new_rgb8(1, 1))),
        };
        let img = meta.take_image();
        assert!(img.is_some());
        assert!(meta.take_image().is_none());
    }

    #[test]
    fn test_get_url_preview_with_meta_and_image() {
        let server = MockServer::start();
        let image_url = format!("{}/image.png", server.base_url());

        let html_mock = server.mock(|when, then| {
            when.method(GET).path("/page");
            then.status(200)
                .header("content-type", "text/html")
                .body(format!(
                    r#"<html><head>
                        <meta property="og:title" content="Mock Title">
                        <meta property="og:image" content="{image_url}">
                    </head></html>"#
                ));
        });

        let img_bytes: &[u8] = b"\x89PNG\r\n\x1a\n\
        \x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\
        \x08\x06\x00\x00\x00\x1f\x15\xc4\x89\
        \x00\x00\x00\nIDATx\x9cc``\x00\x00\x00\x02\x00\x01\
        \xe2!\xbc3\x00\x00\x00\x00IEND\xaeB`\x82";
        let image_mock = server.mock(|when, then| {
            when.method(GET).path("/image.png");
            then.status(200)
                .header("content-type", "image/png")
                .body(img_bytes);
        });

        let url = format!("{}/page", server.base_url());
        let result = get_url_preview(&url);
        assert!(result.is_ok());

        let meta = result.unwrap();
        assert_eq!(meta.get_title(), "Mock Title");
        assert_eq!(meta.image_url, image_url);

        html_mock.assert();
        image_mock.assert();
    }

    #[test]
    fn test_get_url_preview_with_image_direct() {
        let server = MockServer::start();

        let img_bytes = vec![255u8; 10];
        let mock = server.mock(|when, then| {
            when.method(GET).path("/image.png");
            then.status(200)
                .header("content-type", "image/png")
                .body(img_bytes.clone());
        });

        let url = format!("{}/image.png", server.base_url());
        let result = get_url_preview(&url);
        assert!(result.is_ok());

        let meta = result.unwrap();
        assert_eq!(meta.image_url, url);
        assert!(meta.image.is_none());

        mock.assert();
    }

    #[test]
    #[ignore]
    #[allow(clippy::print_stdout)]
    fn test_get_url_preview_real_url() {
        let url = "https://github.com";

        let result = get_url_preview(url);

        assert!(result.is_ok(), "Failed to fetch URL: {:?}", result.err());

        let meta = result.unwrap();
        println!("Title: {}", meta.get_title());
        println!("Image URL: {}", meta.image_url);
        println!("Description: {}", meta.description);

        assert!(!meta.image_url.is_empty());
        assert!(!meta.title.is_empty() || !meta.description.is_empty());
        assert!(meta.image.is_some());
        if let Some(image) = meta.image {
            assert!(image.width() > 0);
            assert!(image.height() > 0);
        }
    }
}
