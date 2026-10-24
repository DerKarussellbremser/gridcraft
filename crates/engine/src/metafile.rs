//! Windows metafiles (EMF, WMF) as pictures. Office often saves a pasted or embedded picture as
//! a metafile that holds one bitmap; GridCraft shows that bitmap. The largest device-independent
//! bitmap the file carries is returned as a BMP file, which the image decoder reads. Metafiles
//! drawn only with vector records give `None` (the picture keeps its placeholder). Layouts follow
//! the published [MS-EMF] and [MS-WMF] specifications. The bytes come from a file: every offset
//! and size is checked.

const EMF_STRETCHDIBITS: u32 = 0x51;
const EMF_SETDIBITSTODEVICE: u32 = 0x50;
const EMF_BITBLT: u32 = 0x4C;
const EMF_STRETCHBLT: u32 = 0x4D;
const WMF_STRETCHDIB: u16 = 0x0F43;
const WMF_DIBSTRETCHBLT: u16 = 0x0B41;
const WMF_DIBBITBLT: u16 = 0x0940;
const WMF_SETDIBTODEV: u16 = 0x0D33;
/// Most records walked: a metafile is input like any other.
const MAX_RECORDS: usize = 1_000_000;

/// The metafile's bitmap as a BMP file, or `None` when `data` isn't an EMF or WMF with one.
pub fn raster(data: &[u8]) -> Option<Vec<u8>> {
    let (info, bits) = if is_emf(data) { emf_bitmap(data)? } else { wmf_bitmap(data)? };
    bmp(info, bits)
}

/// Whether `data` is an EMF or WMF (by its header), so callers can tell a metafile they couldn't
/// show from a broken picture.
pub fn is_metafile(data: &[u8]) -> bool {
    is_emf(data) || wmf_records_start(data).is_some()
}

fn u16_at(b: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(i..i.checked_add(2)?)?.try_into().ok()?))
}

fn u32_at(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(i..i.checked_add(4)?)?.try_into().ok()?))
}

/// EMR_HEADER (type 1) with the " EMF" signature at offset 40.
fn is_emf(b: &[u8]) -> bool {
    u32_at(b, 0) == Some(1) && b.get(40..44) == Some(b" EMF")
}

/// The largest bitmap of an EMF: (BITMAPINFO, pixel bits), which EMF stores apart.
fn emf_bitmap(b: &[u8]) -> Option<(&[u8], &[u8])> {
    let mut best: Option<(&[u8], &[u8])> = None;
    let mut i = 0usize;
    for _ in 0..MAX_RECORDS {
        let (Some(kind), Some(size)) = (u32_at(b, i), u32_at(b, i + 4)) else { break };
        let size = size as usize;
        if size < 8 {
            break;
        }
        let record = b.get(i..i.checked_add(size)?)?;
        // Where the BITMAPINFO and bits offsets sit in each record (offBmiSrc, cbBmiSrc,
        // offBitsSrc, cbBitsSrc follow each other).
        let at = match kind {
            EMF_STRETCHDIBITS | EMF_SETDIBITSTODEVICE => Some(48),
            EMF_BITBLT | EMF_STRETCHBLT => Some(84),
            _ => None,
        };
        if let Some(at) = at
            && let (Some(off_bmi), Some(cb_bmi), Some(off_bits), Some(cb_bits)) =
                (u32_at(record, at), u32_at(record, at + 4), u32_at(record, at + 8), u32_at(record, at + 12))
            && let (Some(info), Some(bits)) = (slice(record, off_bmi, cb_bmi), slice(record, off_bits, cb_bits))
            && !info.is_empty()
            && best.is_none_or(|(_, b)| bits.len() > b.len())
        {
            best = Some((info, bits));
        }
        i = i.checked_add(size)?;
    }
    best
}

fn slice(b: &[u8], off: u32, len: u32) -> Option<&[u8]> {
    let off = off as usize;
    b.get(off..off.checked_add(len as usize)?)
}

/// Where the records of a WMF start: after the optional placeable header and the META_HEADER.
fn wmf_records_start(b: &[u8]) -> Option<usize> {
    let base = if u32_at(b, 0) == Some(0x9AC6_CDD7) { 22 } else { 0 };
    // META_HEADER: type 1 (memory) or 2 (disk), header size 9 words, a version.
    let (kind, words, version) = (u16_at(b, base)?, u16_at(b, base + 2)?, u16_at(b, base + 4)?);
    ((kind == 1 || kind == 2) && words == 9 && (version == 0x0100 || version == 0x0300)).then_some(base + 18)
}

/// The largest bitmap of a WMF, split into its BITMAPINFO and bits (WMF stores them together).
fn wmf_bitmap(b: &[u8]) -> Option<(&[u8], &[u8])> {
    let mut i = wmf_records_start(b)?;
    let mut best: Option<&[u8]> = None;
    for _ in 0..MAX_RECORDS {
        let (Some(words), Some(function)) = (u32_at(b, i), u16_at(b, i + 4)) else { break };
        let size = (words as usize).checked_mul(2)?;
        if size < 6 || function == 0 {
            break;
        }
        let record = b.get(i..i.checked_add(size)?)?;
        // Where the DIB starts in each record, after its fixed fields.
        let at = match function {
            WMF_STRETCHDIB => Some(28),
            WMF_DIBSTRETCHBLT => Some(26),
            WMF_DIBBITBLT => Some(22),
            WMF_SETDIBTODEV => Some(24),
            _ => None,
        };
        if let Some(dib) = at.and_then(|at| record.get(at..))
            && dib.len() > 40
            && best.is_none_or(|b| dib.len() > b.len())
        {
            best = Some(dib);
        }
        i = i.checked_add(size)?;
    }
    let dib = best?;
    let info_len = info_size(dib)?;
    Some((dib.get(..info_len)?, dib.get(info_len..)?))
}

/// Size of a packed DIB's BITMAPINFO: the header, the colour table and any bit-field masks.
fn info_size(dib: &[u8]) -> Option<usize> {
    let header = u32_at(dib, 0)? as usize;
    if header < 12 {
        return None;
    }
    if header == 12 {
        // BITMAPCOREHEADER: 3-byte palette entries for 1, 4 and 8 bits per pixel.
        let bits = u16_at(dib, 10)?;
        return Some(12 + if bits <= 8 { 3usize << bits } else { 0 });
    }
    let bits = u16_at(dib, 14)?;
    let compression = u32_at(dib, 16)?;
    let used = u32_at(dib, 32).unwrap_or(0) as usize;
    let colors = if used > 0 {
        used.min(256)
    } else if bits <= 8 {
        1usize << bits
    } else {
        0
    };
    // BI_BITFIELDS (3) and BI_ALPHABITFIELDS (6) put their masks after a 40-byte header.
    let masks = match (header, compression) {
        (40, 3) => 12,
        (40, 6) => 16,
        _ => 0,
    };
    Some(header + masks + colors * 4)
}

/// A BMP file: BITMAPFILEHEADER, then the BITMAPINFO and the bits.
fn bmp(info: &[u8], bits: &[u8]) -> Option<Vec<u8>> {
    let offset = 14usize.checked_add(info.len())?;
    let total = offset.checked_add(bits.len())?;
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&u32::try_from(total).ok()?.to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&u32::try_from(offset).ok()?.to_le_bytes());
    out.extend_from_slice(info);
    out.extend_from_slice(bits);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 2×1 24-bit DIB: header (40 bytes), then one row padded to 4 bytes (blue, red).
    fn dib() -> Vec<u8> {
        let mut d = Vec::new();
        for v in [40u32, 2, 1] {
            d.extend_from_slice(&v.to_le_bytes());
        }
        d.extend_from_slice(&1u16.to_le_bytes());
        d.extend_from_slice(&24u16.to_le_bytes());
        d.extend_from_slice(&[0; 24]);
        d.extend_from_slice(&[255, 0, 0, 0, 0, 255, 0, 0]);
        d
    }

    fn pixels(bmp: &[u8]) -> Vec<[u8; 4]> {
        let img = image::load_from_memory(bmp).unwrap().to_rgba8();
        img.pixels().map(|p| p.0).collect()
    }

    #[test]
    fn a_wmf_stretchdib_becomes_its_bitmap() {
        let dib = dib();
        let mut wmf = Vec::new();
        for v in [1u16, 9, 0x0300] {
            wmf.extend_from_slice(&v.to_le_bytes());
        }
        wmf.extend_from_slice(&[0; 12]);
        let record_len = 28 + dib.len();
        wmf.extend_from_slice(&((record_len / 2) as u32).to_le_bytes());
        wmf.extend_from_slice(&WMF_STRETCHDIB.to_le_bytes());
        wmf.extend_from_slice(&[0; 22]);
        wmf.extend_from_slice(&dib);
        wmf.extend_from_slice(&3u32.to_le_bytes());
        wmf.extend_from_slice(&0u16.to_le_bytes());
        assert!(is_metafile(&wmf));
        assert_eq!(pixels(&raster(&wmf).unwrap()), [[0, 0, 255, 255], [255, 0, 0, 255]]);
    }

    #[test]
    fn an_emf_stretchdibits_becomes_its_bitmap() {
        let dib = dib();
        let (info, bits) = dib.split_at(40);
        let mut emf = vec![0u8; 88];
        emf[0..4].copy_from_slice(&1u32.to_le_bytes());
        emf[4..8].copy_from_slice(&88u32.to_le_bytes());
        emf[40..44].copy_from_slice(b" EMF");
        let mut rec = vec![0u8; 80];
        rec[0..4].copy_from_slice(&EMF_STRETCHDIBITS.to_le_bytes());
        let size = 80 + info.len() + bits.len();
        rec[4..8].copy_from_slice(&(size as u32).to_le_bytes());
        rec[48..52].copy_from_slice(&80u32.to_le_bytes());
        rec[52..56].copy_from_slice(&(info.len() as u32).to_le_bytes());
        rec[56..60].copy_from_slice(&((80 + info.len()) as u32).to_le_bytes());
        rec[60..64].copy_from_slice(&(bits.len() as u32).to_le_bytes());
        rec.extend_from_slice(info);
        rec.extend_from_slice(bits);
        emf.extend_from_slice(&rec);
        assert_eq!(pixels(&raster(&emf).unwrap()), [[0, 0, 255, 255], [255, 0, 0, 255]]);
    }

    #[test]
    fn other_data_and_damaged_metafiles_give_nothing() {
        assert_eq!(raster(b"\x89PNG\r\n\x1a\n"), None);
        assert!(!is_metafile(b"\x89PNG\r\n\x1a\n"));
        let mut emf = vec![0u8; 100];
        emf[0] = 1;
        emf[40..44].copy_from_slice(b" EMF");
        emf[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(raster(&emf), None);
    }
}
