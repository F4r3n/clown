use image::DynamicImage;
use scraper::Html;
use std::io::Cursor;
use std::sync::Arc;
#[derive(Clone)]
pub struct MetaData {
    image_url: String,
    title: String,
    description: String,
    site: String,
    image: Option<Arc<DynamicImage>>,
}

impl MetaData {
    pub fn new(in_html: Html) -> Self {
        let mut meta = MetaData {
            image_url: String::from(""),
            title: String::from(""),
            description: String::from(""),
            site: String::from(""),
            image: None,
        };
        use scraper::Selector;
        if let Ok(selector) = Selector::parse("head meta") {
            let s = in_html.select(&selector);
            for element in s {
                if let Some(property) = element.attr("property")
                    && let Some(content) = element.attr("content")
                {
                    match property {
                        "og:title" => {
                            meta.title = String::from(content);
                        }
                        "og:image" => {
                            meta.image_url = String::from(content);
                        }
                        "og:description" => {
                            meta.description = String::from(content);
                        }
                        "og:site" => {
                            meta.site = String::from(content);
                        }
                        _ => {}
                    }
                }
            }
        }
        meta
    }

    pub fn get_title(&self) -> &str {
        &self.title
    }

    pub fn take_image(&mut self) -> Option<Arc<DynamicImage>> {
        self.image.take()
    }
}

fn parse_html(text: &str, is_meta: bool) -> MetaData {
    if !is_meta {
        return MetaData {
            image_url: String::new(),
            title: String::new(),
            description: String::new(),
            site: String::new(),
            image: None,
        };
    }
    let document = Html::parse_document(text);
    MetaData::new(document)
}

pub async fn get_url_preview(endpoint: &str) -> Result<MetaData, String> {
    let client = reqwest::Client::new();
    let resp = client
        .get(endpoint)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let headers = resp.headers();
    let has_meta = headers
        .get("content-type")
        .and_then(|ct| ct.to_str().ok())
        .map_or_else(|| false, |ct| !ct.starts_with("image"));

    let mut bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    let mut metadata = if has_meta {
        let text = String::from_utf8_lossy(&bytes);
        parse_html(&text, has_meta)
    } else {
        MetaData {
            image_url: String::from(endpoint),
            title: String::from(""),
            description: String::from(""),
            site: String::from(""),
            image: None,
        }
    };
    if metadata.image_url != endpoint {
        bytes = client
            .get(metadata.image_url.clone())
            .send()
            .await
            .map_err(|e| e.to_string())?
            .bytes()
            .await
            .map_err(|e| e.to_string())?;
    }

    if let Ok(image_reader) = image::ImageReader::new(Cursor::new(bytes)).with_guessed_format()
        && let Ok(image) = image_reader.decode()
    {
        metadata.image = Some(Arc::new(image));
    }

    Ok(metadata)
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use scraper::Html;
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

        let doc = Html::parse_document(html);
        let meta = MetaData::new(doc);

        assert_eq!(meta.title, "Example Title");
        assert_eq!(meta.description, "Example description.");
        assert_eq!(meta.image_url, "https://example.com/image.jpg");
        assert_eq!(meta.site, "ExampleSite");
        assert!(meta.image.is_none());
    }

    #[test]
    fn test_parse_html_no_meta() {
        let html = "<html><body>No meta here</body></html>";
        let meta = parse_html(html, true);
        assert_eq!(meta.title, "");
        assert_eq!(meta.image_url, "");
    }

    #[test]
    fn test_parse_html_disabled_meta_flag() {
        let html = "<html><head><meta property='og:title' content='Ignored'></head></html>";
        let meta = parse_html(html, false);
        assert_eq!(meta.title, "");
    }

    #[test]
    fn test_take_image() {
        let mut meta = MetaData {
            image_url: String::from(""),
            title: String::from(""),
            description: String::from(""),
            site: String::from(""),
            image: Some(Arc::new(DynamicImage::new_rgb8(1, 1))),
        };
        let img = meta.take_image();
        assert!(img.is_some());
        assert!(meta.take_image().is_none()); // should now be taken
    }

    #[tokio::test]
    async fn test_get_url_preview_with_meta_and_image() {
        let server = MockServer::start();
        let image_url = format!("{}/image.png", server.base_url());
        // Mock HTML response with OG tags
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

        // Mock image response
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
        let result = get_url_preview(&url).await;
        assert!(result.is_ok());

        let meta = result.unwrap();
        assert_eq!(meta.get_title(), "Mock Title");
        assert_eq!(meta.image_url, image_url);

        html_mock.assert();
        image_mock.assert();
    }

    #[tokio::test]
    async fn test_get_url_preview_with_image_direct() {
        let server = MockServer::start();

        // Mock direct image URL
        let img_bytes = vec![255u8; 10];
        let mock = server.mock(|when, then| {
            when.method(GET).path("/image.png");
            then.status(200)
                .header("content-type", "image/png")
                .body(img_bytes.clone());
        });

        let url = format!("{}/image.png", server.base_url());
        let result = get_url_preview(&url).await;
        assert!(result.is_ok());

        let meta = result.unwrap();
        assert_eq!(meta.image_url, url);
        assert!(meta.image.is_none());

        mock.assert();
    }

    #[tokio::test]
    #[ignore] // ignored by default
    async fn test_get_url_preview_real_url() {
        let url = "https://github.com";

        let result = get_url_preview(url).await;

        assert!(result.is_ok(), "Failed to fetch URL: {:?}", result.err());

        let meta = result.unwrap();
        println!("Title: {}", meta.get_title());
        println!("Image URL: {}", meta.image_url);
        println!("Description: {}", meta.description);
        println!("Site: {}", meta.site);

        // Basic sanity checks
        assert!(!meta.image_url.is_empty());
        assert!(!meta.title.is_empty() || !meta.description.is_empty());
        assert!(meta.image.is_some());
        if let Some(image) = meta.image {
            assert!(image.width() > 0);
            assert!(image.height() > 0);
        }
    }
}
