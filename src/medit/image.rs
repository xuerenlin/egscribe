
use eframe::egui::{ColorImage, Color32};
use arboard::Clipboard;
use image::{ImageBuffer, ImageResult, RgbaImage};

#[derive(Clone, Debug)]
pub struct ImageInfo {
    pub alt: String,
    pub url: String,
    pub img: Option<ColorImage>
}

impl ImageInfo {
    pub fn clipboard_to_file(clipboard: &mut Clipboard, alt: String, url: String, path: String) -> Option<Self> {
        if let Ok(image_data) = clipboard.get_image() {
            let pixels: Vec<_> = image_data
                .bytes
                .chunks_exact(4)  
                .map(|p| Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
                .collect();
    
            let color_image =  ColorImage {
                    size: [image_data.width, image_data.height],
                    pixels,
                };

            //save to file
            println!("clipboard get image ok");
            let rgba_image = Self::color_image_to_rgba(&color_image);
            let _ = Self::save_as_png(rgba_image, &path);

            let image_info = ImageInfo {
                alt,
                url,
                img: Some(color_image)
            };
            return Some(image_info);
        }

        None
    }

    fn color_image_to_rgba(color_image: &ColorImage) -> RgbaImage {
        let pixels = color_image.pixels.iter().map(|c| {
            image::Rgba([c.r(), c.g(), c.b(), c.a()])
        }).collect::<Vec<_>>();
    
        ImageBuffer::from_vec(
            color_image.size[0] as u32,
            color_image.size[1] as u32,
            pixels.iter().flat_map(|p| p.0).collect()
        ).unwrap()
    }

    fn save_as_png(rgba_image: RgbaImage, path: &str) -> ImageResult<()> {
        rgba_image.save(path)?;
        Ok(())
    }
}


/* 
fn test_clipboard(ui: &mut Ui) {
	let mut clipboard: Clipboard = Clipboard::new().unwrap();

    //if let Ok(text) = clipboard.get_text() {
    //    println!("Clipboard text was: {:?}", text);
    //}

    if let Ok(image_data) = clipboard.get_image() {
        //println!("Image data is:\n{:?}", image_data.bytes);
        let pixels: Vec<_> = image_data
            .bytes
            .chunks_exact(4)  // 假设是 RGBA 格式（4字节/像素）
            .map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
            .collect();

        let color_image =  ColorImage {
                size: [image_data.width, image_data.height],
                pixels,
            };
        let copy_iamge = color_image.clone();
        let copy_bytes = image_data.bytes.clone();

        let texture = ui.ctx().load_texture(
            "clipboard_image",
            color_image,
            Default::default()
        );
        let img = Image::new(&texture).max_width(200.0);
        ui.add(img);


        //set to clipboard again
        let new_image_data = ImageData { width: copy_iamge.width(), height: copy_iamge.height(), bytes: copy_bytes};
        if clipboard.set_image(new_image_data).is_ok() {
            println!("set imaget ok");
        }
    }
    

    //#[rustfmt::skip]
	//let bytes = [
	//	255, 100, 100, 255,
	//	100, 255, 100, 100,
	//	100, 100, 255, 100,
	//	0, 0, 0, 255,
	//];
	//let img_data = ImageData { width: 2, height: 2, bytes: bytes.as_ref().into() };
	//ctx.set_image(img_data).unwrap();
}
*/