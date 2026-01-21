use eyre::ensure;
use eyre::eyre;
use image::RgbaImage;
use std::ops::Deref;
use windows::Win32::Graphics::Gdi::BI_RGB;
use windows::Win32::Graphics::Gdi::BITMAP;
use windows::Win32::Graphics::Gdi::BITMAPINFO;
use windows::Win32::Graphics::Gdi::BITMAPINFOHEADER;
use windows::Win32::Graphics::Gdi::CreateCompatibleDC;
use windows::Win32::Graphics::Gdi::DIB_RGB_COLORS;
use windows::Win32::Graphics::Gdi::DeleteDC;
use windows::Win32::Graphics::Gdi::GetDC;
use windows::Win32::Graphics::Gdi::GetDIBits;
use windows::Win32::Graphics::Gdi::GetObjectW;
use windows::Win32::Graphics::Gdi::HDC;
use windows::Win32::Graphics::Gdi::HGDIOBJ;
use windows::Win32::Graphics::Gdi::ReleaseDC;
use windows::Win32::Graphics::Gdi::SelectObject;
use windows::Win32::UI::WindowsAndMessaging::GetIconInfo;
use windows::Win32::UI::WindowsAndMessaging::HICON;
use windows::Win32::UI::WindowsAndMessaging::ICONINFO;
use windows::core::Owned;

pub unsafe fn hicon_to_rgba(hicon: HICON) -> eyre::Result<RgbaImage> {
    // Get the ICONINFO from the HICON
    let mut icon_info = ICONINFO::default();
    unsafe { GetIconInfo(hicon, &mut icon_info) }?;

    // Destructure ICONINFO
    let ICONINFO {
        hbmMask,  // must be cleaned up
        hbmColor, // must be cleaned up
        ..
    } = icon_info;

    // Move into RAII guards for automatic cleanup
    let hbm_mask = unsafe { Owned::new(hbmMask) };
    let hbm_color = unsafe { Owned::new(hbmColor) };

    // Get bitmap info for hbmColor
    let mut bitmap = BITMAP::default();
    ensure!(
        unsafe {
            GetObjectW(
                HGDIOBJ::from(*hbm_color),
                std::mem::size_of::<BITMAP>() as i32,
                Some(&raw mut bitmap as *mut _),
            )
        } != 0,
        "GetObjectW failed to get bitmap info"
    );

    // Determine width and height
    let width = u32::try_from(bitmap.bmWidth)?;
    let height = u32::try_from(bitmap.bmHeight)?;
    ensure!(width > 0, "Bitmap width must not be zero");
    ensure!(height > 0, "Bitmap height must not be zero");

    // Create a compatible DC
    let screen_device_context = ReleaseDCGuard(unsafe { GetDC(None) });

    let memory_device_context =
        DeleteDCGuard(unsafe { CreateCompatibleDC(Some(*screen_device_context)) });

    let old_bitmap = unsafe { SelectObject(*memory_device_context, HGDIOBJ::from(*hbm_color)) };

    let _old_bitmap_guard = SelectObjectGuard(*memory_device_context, old_bitmap);

    let mut bitmap_info = BITMAPINFO::default();
    bitmap_info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bitmap_info.bmiHeader.biWidth = width as i32;
    bitmap_info.bmiHeader.biHeight = -(height as i32); // top-down
    bitmap_info.bmiHeader.biPlanes = 1;
    bitmap_info.bmiHeader.biBitCount = 32; // RGBA
    bitmap_info.bmiHeader.biCompression = BI_RGB.0 as u32;

    let mut image_data = vec![0u8; (width * height * 4) as usize];
    ensure!(
        unsafe {
            GetDIBits(
                *memory_device_context,
                *hbm_color,
                0,
                height as u32,
                Some(image_data.as_mut_ptr() as *mut _),
                &mut bitmap_info,
                DIB_RGB_COLORS,
            ) != 0
        },
        "GetDIBits failed to get bitmap bits"
    );

    if !hbm_mask.is_invalid() && hbm_mask != hbm_color {
        // conversion is necessary

        let mut mask_bitmap_info = BITMAPINFO::default();
        mask_bitmap_info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        mask_bitmap_info.bmiHeader.biWidth = width as i32;
        mask_bitmap_info.bmiHeader.biHeight = -(height as i32); // top-down
        mask_bitmap_info.bmiHeader.biPlanes = 1;
        mask_bitmap_info.bmiHeader.biBitCount = 1; // 1-bit mask/per pixel
        mask_bitmap_info.bmiHeader.biCompression = BI_RGB.0 as u32;

        let mut mask_pixel_data = vec![0u8; (width * height) as usize]; // 1bpp mask data
        ensure!(
            unsafe {
                GetDIBits(
                    *memory_device_context,
                    *hbm_mask,
                    0,
                    height as u32,
                    Some(mask_pixel_data.as_mut_ptr() as *mut _),
                    &mut mask_bitmap_info,
                    DIB_RGB_COLORS,
                ) != 0
            },
            "GetDIBits failed to get mask bitmap bits"
        );

        // Apply the mask to the RGBA data
        // Iterate over each pixel. The mask_pixel_data is packed 8 pixels per byte.
        // The DIB is top-down, so scanlines are in order.
        // Row size for 1bpp DIB must be DWORD aligned.
        let row_size_bytes = ((width + 31) / 32) * 4;

        for y in 0..height {
            for x in 0..width {
                let byte_index = (y * row_size_bytes + x / 8) as usize;
                let bit_index = 7 - (x % 8); // Bits are packed from MSB to LSB
                let mask_bit = (mask_pixel_data[byte_index] >> bit_index) & 1;

                let pixel_idx_rgba = ((y * width + x) * 4) as usize;
                if mask_bit == 1 {
                    // Mask bit 1 means transparent (for ICONINFO mask)
                    image_data[pixel_idx_rgba + 3] = 0; // Set alpha to transparent
                } else {
                    // Mask bit 0 means opaque
                    if bitmap.bmBitsPixel != 32 {
                        // If original wasn't 32bpp, ensure opaque
                        image_data[pixel_idx_rgba + 3] = 255;
                    }
                    // If original was 32bpp, its alpha is already in image_data[pixel_idx_rgba + 3]
                    // and this mask bit being 0 means that alpha should be preserved.
                }
            }
        }
    } else {
        // No separate mask bitmap; if original wasn't 32bpp, set alpha to opaque
        if bitmap.bmBitsPixel != 32 {
            for i in 0..(width * height) as usize {
                image_data[i * 4 + 3] = 255;
            }
        }
    }

    for i in 0..(width * height) as usize {
        let pixel_idx = i * 4;
        image_data.swap(pixel_idx, pixel_idx + 2); // BGRA to RGBA
    }

    Ok(
        RgbaImage::from_raw(width, height, image_data).ok_or_else(|| {
            eyre!(
                "Failed to create RgbaImage from raw data with width {} and height {}",
                width,
                height
            )
        })?,
    )
}

/// Release on drop
pub struct ReleaseDCGuard(pub HDC);
impl Drop for ReleaseDCGuard {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe { _ = ReleaseDC(None, self.0) };
        }
    }
}
impl Deref for ReleaseDCGuard {
    type Target = HDC;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Delete on drop
pub struct DeleteDCGuard(pub HDC);
impl Drop for DeleteDCGuard {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe { _ = DeleteDC(self.0) };
        }
    }
}
impl Deref for DeleteDCGuard {
    type Target = HDC;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Select on drop
pub struct SelectObjectGuard(pub HDC, pub HGDIOBJ);
impl Drop for SelectObjectGuard {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe { _ = SelectObject(self.0, self.1) };
        }
    }
}
