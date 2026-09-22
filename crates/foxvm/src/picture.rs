//! Pictures a program has loaded, and the two numbers a program reads off one.
//!
//! `LOADPICTURE()` answers with an object holding a Handle, a Type and a size; `SAVEPICTURE()`
//! takes that object back and writes a bitmap. The pixels behind the handle live here, in the
//! VM, because both hosts would otherwise need a decoder of their own. Decoding and encoding
//! are the `image` crate's; nothing here reads a file format byte by byte.

use crate::error::RtError;

/// A picture that has been loaded, as the running program can still reach it.
pub struct Picture {
    /// What `LOADPICTURE()` answers for `Type`: 1 a bitmap, 2 a metafile, 3 an icon, 4 an
    /// enhanced metafile. Everything the `image` crate reads is a bitmap once it is decoded.
    pub kind: u8,
    image: image::RgbaImage,
}

/// Windows measures a picture in HIMETRIC units - hundredths of a millimetre - and that is what
/// `Width` and `Height` answer with. Ninety-six pixels to the inch is what the product assumes,
/// measured: a 32-pixel picture answers 847, which is 32 x 2540 / 96 rounded.
const HIMETRIC_PER_INCH: f64 = 2540.0;
const PIXELS_PER_INCH: f64 = 96.0;

fn himetric(pixels: u32) -> f64 {
    (f64::from(pixels) * HIMETRIC_PER_INCH / PIXELS_PER_INCH).round()
}

impl Picture {
    /// Reads a picture out of the bytes of a file, whatever format they are in.
    pub fn decode(bytes: &[u8]) -> Result<Picture, RtError> {
        let decoded = image::load_from_memory(bytes)
            .map_err(|e| RtError::new(RtError::OLE_ERROR, format!("The picture cannot be read: {e}")))?;
        Ok(Picture { kind: 1, image: decoded.to_rgba8() })
    }

    /// The bitmap `SAVEPICTURE()` writes.
    pub fn to_bitmap(&self) -> Result<Vec<u8>, RtError> {
        let mut out = std::io::Cursor::new(Vec::new());
        self.image
            .write_to(&mut out, image::ImageFormat::Bmp)
            .map_err(|e| RtError::new(RtError::OLE_ERROR, format!("The picture cannot be written: {e}")))?;
        Ok(out.into_inner())
    }

    pub fn width(&self) -> f64 {
        himetric(self.image.width())
    }

    pub fn height(&self) -> f64 {
        himetric(self.image.height())
    }
}

/// The pictures one running program has loaded.
///
/// A handle is the 1-based place in this list, so it is a small number a program can hold in a
/// property and hand back. Handle 0 is the null picture - what `LOADPICTURE()` with no file
/// answers with - and there is nothing behind it.
#[derive(Default)]
pub struct Pictures(Vec<Picture>);

impl Pictures {
    pub fn keep(&mut self, picture: Picture) -> f64 {
        self.0.push(picture);
        self.0.len() as f64
    }

    pub fn get(&self, handle: f64) -> Option<&Picture> {
        let at = handle as i64;
        (at > 0).then(|| self.0.get(at as usize - 1)).flatten()
    }
}
