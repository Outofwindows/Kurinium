use anyhow::{Context, Result};
use image::{ImageBuffer, Rgba};
use std::io::Cursor;
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits,
    ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, SRCCOPY,
};
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

pub struct Screenshot;

impl Screenshot {
    pub fn capture_as_bytes() -> Result<(Vec<u8>, String)> {
        Self::capture_as_bytes_native()
    }

    fn capture_as_bytes_native() -> Result<(Vec<u8>, String)> {
        unsafe {
            let width = GetSystemMetrics(SM_CXSCREEN);
            let height = GetSystemMetrics(SM_CYSCREEN);

            if width <= 0 || height <= 0 {
                return Err(anyhow::anyhow!("Invalid screen dimensions: {}x{}", width, height));
            }

            let screen_dc = GetDC(None);
            if screen_dc.is_invalid() {
                return Err(anyhow::anyhow!("Failed to get screen DC"));
            }

            let mem_dc = CreateCompatibleDC(Some(screen_dc));
            if mem_dc.is_invalid() {
                ReleaseDC(None, screen_dc);
                return Err(anyhow::anyhow!("Failed to create compatible DC"));
            }

            let bitmap = CreateCompatibleBitmap(screen_dc, width, height);
            if bitmap.is_invalid() {
                let _ = DeleteDC(mem_dc);
                ReleaseDC(None, screen_dc);
                return Err(anyhow::anyhow!("Failed to create bitmap"));
            }

            let old_bitmap = SelectObject(mem_dc, bitmap.into());

            let blt_result = BitBlt(mem_dc, 0, 0, width, height, Some(screen_dc), 0, 0, SRCCOPY);
            if blt_result.is_err() {
                SelectObject(mem_dc, old_bitmap);
                let _ = DeleteObject(bitmap.into());
                let _ = DeleteDC(mem_dc);
                ReleaseDC(None, screen_dc);
                return Err(anyhow::anyhow!("BitBlt failed"));
            }

            let mut bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    biSizeImage: 0,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [Default::default()],
            };

            let mut buffer: Vec<u8> = vec![0u8; (width * height * 4) as usize];

            let lines = GetDIBits(
                mem_dc,
                bitmap,
                0,
                height as u32,
                Some(buffer.as_mut_ptr() as *mut _),
                &mut bmi,
                DIB_RGB_COLORS,
            );

            SelectObject(mem_dc, old_bitmap);
            let _ = DeleteObject(bitmap.into());
            let _ = DeleteDC(mem_dc);
            ReleaseDC(None, screen_dc);

            if lines == 0 {
                return Err(anyhow::anyhow!("GetDIBits failed"));
            }

            for chunk in buffer.chunks_exact_mut(4) {
                chunk.swap(0, 2);
            }

            let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
                ImageBuffer::from_raw(width as u32, height as u32, buffer)
                    .context("Failed to create image buffer")?;

            let mut png_buffer = Cursor::new(Vec::new());
            img.write_to(&mut png_buffer, image::ImageFormat::Png)
                .context("Failed to encode PNG")?;

            let random_id: u64 = rand::random();
            let filename = format!("screenshot_{:x}.png", random_id);

            Ok((png_buffer.into_inner(), filename))
        }
    }
}

pub fn capture_screen() -> Result<Vec<u8>> {
    let (buffer, _) = Screenshot::capture_as_bytes()?;
    Ok(buffer)
}
