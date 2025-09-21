//! Baseline JPEG parser: extract DQT/DHT/DRI and SOF0/SOS templates,
//! and compute (head_len, scan_len) slices for an already-encoded image.

use crate::encode::blp::jpeg::types::*;
use crate::err::error::BlpError;

#[inline]
fn be_u16(b: &[u8]) -> Result<u16, BlpError> {
    if b.len() < 2 {
        return Err(BlpError::new("jpeg_truncated"));
    }
    Ok(u16::from_be_bytes([b[0], b[1]]))
}

pub fn split_header_and_scan(jpeg: &[u8]) -> Result<JpegSlices, BlpError> {
    if jpeg.len() < 4 {
        return Err(BlpError::new("jpeg_too_short"));
    }
    if jpeg[0] != 0xFF || jpeg[1] != 0xD8 {
        return Err(BlpError::new("jpeg_missing_soi"));
    }

    let mut i = 2;
    let head_end = loop {
        if i >= jpeg.len() {
            return Err(BlpError::new("jpeg_truncated_before_sos"));
        }
        if jpeg[i] != 0xFF {
            return Err(BlpError::new("jpeg_bad_marker_alignment"));
        }
        while i < jpeg.len() && jpeg[i] == 0xFF {
            i += 1;
        }
        if i >= jpeg.len() {
            return Err(BlpError::new("jpeg_truncated_before_sos"));
        }
        let m = jpeg[i];
        i += 1;
        match m {
            0xDA => {
                // SOS
                if i + 2 > jpeg.len() {
                    return Err(BlpError::new("jpeg_truncated_sos_len"));
                }
                let len = be_u16(&jpeg[i..i + 2])? as usize;
                let sos_hdr_end = i + len;
                if sos_hdr_end > jpeg.len() {
                    return Err(BlpError::new("jpeg_truncated_sos"));
                }
                break sos_hdr_end;
            }
            0xD9 => return Err(BlpError::new("jpeg_eoi_before_sos")),
            0xC2 => return Err(BlpError::new("jpeg_progressive_unsupported")),              // SOF2
            0xC9 | 0xCA | 0xCB => return Err(BlpError::new("jpeg_arithmetic_unsupported")), // SOF9..SOF11
            0xDC => return Err(BlpError::new("jpeg_dnl_unsupported")),                      // DNL (height after scan)
            0x01 | 0xD0..=0xD7 => { /* standalone before SOS; advance already done */ }
            _ => {
                let seg_len = be_u16(&jpeg[i..i + 2])? as usize;
                if i + seg_len > jpeg.len() {
                    return Err(BlpError::new("jpeg_truncated_segment"));
                }
                i += seg_len;
            }
        }
    };

    // find EOI scanning entropy-coded data
    let scan_start = head_end;
    let mut p = scan_start;
    while p + 1 < jpeg.len() {
        if jpeg[p] != 0xFF {
            p += 1;
            continue;
        }
        let mut q = p + 1;
        while q < jpeg.len() && jpeg[q] == 0xFF {
            q += 1;
        }
        if q >= jpeg.len() {
            return Err(BlpError::new("jpeg_truncated_in_scan"));
        }
        match jpeg[q] {
            0x00 => {
                p = q + 1; // stuffed FF
            }
            0x01 => {
                p = q + 1; // TEM: standalone, can appear in scan
            }
            0xD0..=0xD7 => {
                p = q + 1; // restart marker
            }
            0xD9 => {
                let scan_len = p - scan_start;
                if scan_start + scan_len + 2 != jpeg.len() {
                    return Err(BlpError::new("jpeg_size_mismatch"));
                }
                return Ok(JpegSlices { head_len: scan_start, scan_len });
            }
            _ => return Err(BlpError::new("jpeg_unexpected_marker_in_scan")),
        }
    }
    Err(BlpError::new("jpeg_eoi_not_found"))
}

/// Extract tables and templates needed to rebuild headers.
/// Assumes **baseline** single-scan JPEG.
pub fn extract_plan(jpeg: &[u8]) -> Result<JpegPlan, BlpError> {
    if jpeg.len() < 4 {
        return Err(BlpError::new("jpeg_too_short"));
    }
    if jpeg[0] != 0xFF || jpeg[1] != 0xD8 {
        return Err(BlpError::new("jpeg_missing_soi"));
    }

    let mut dqt_map: [Option<[u8; 64]>; 4] = [None, None, None, None];
    let mut dht_dc: [Option<([u8; 16], Vec<u8>)>; 4] = [None, None, None, None];
    let mut dht_ac: [Option<([u8; 16], Vec<u8>)>; 4] = [None, None, None, None];
    let mut dri = Dri::default();
    let mut sof0: Option<Sof0Template> = None;
    let mut sos: Option<SosTemplate> = None;
    let mut app_segments = Vec::<AppSegment>::new();

    let mut i = 2;
    while i < jpeg.len() {
        if jpeg[i] != 0xFF {
            return Err(BlpError::new("jpeg_bad_marker_alignment"));
        }
        while i < jpeg.len() && jpeg[i] == 0xFF {
            i += 1;
        }
        if i >= jpeg.len() {
            break;
        }
        let m = jpeg[i];
        i += 1;

        match m {
            0xD9 => break,                                                                  // EOI
            0xC2 => return Err(BlpError::new("jpeg_progressive_unsupported")),              // SOF2
            0xC9 | 0xCA | 0xCB => return Err(BlpError::new("jpeg_arithmetic_unsupported")), // SOF9..SOF11
            0xDC => return Err(BlpError::new("jpeg_dnl_unsupported")),
            0x01 | 0xD0..=0xD7 => { /* standalone; unexpected before SOS but tolerate */ }
            _ => {
                if i + 2 > jpeg.len() {
                    return Err(BlpError::new("jpeg_truncated_seg_len"));
                }
                let seg_len = be_u16(&jpeg[i..i + 2])? as usize;
                let seg_end = i + seg_len;
                if seg_end > jpeg.len() {
                    return Err(BlpError::new("jpeg_truncated_segment"));
                }
                let body = &jpeg[i + 2..seg_end];

                match m {
                    0xDB => {
                        // DQT (may contain multiple tables)
                        let mut p = 0;
                        while p < body.len() {
                            let pq_tq = body[p];
                            p += 1;
                            let pq = pq_tq >> 4;
                            let tq = pq_tq & 0x0F;
                            if pq != 0 {
                                return Err(BlpError::new("jpeg_16bit_quant_unsupported"));
                            }
                            if p + 64 > body.len() {
                                return Err(BlpError::new("jpeg_truncated_dqt"));
                            }
                            let mut arr = [0u8; 64];
                            arr.copy_from_slice(&body[p..p + 64]);
                            p += 64;
                            if (tq as usize) < 4 {
                                dqt_map[tq as usize] = Some(arr);
                            }
                        }
                    }
                    0xC4 => {
                        // DHT (may contain multiple tables)
                        let mut p = 0;
                        while p < body.len() {
                            if p >= body.len() {
                                return Err(BlpError::new("jpeg_truncated_dht"));
                            }
                            let tc_th = body[p];
                            p += 1;
                            let class_dc = (tc_th >> 4) == 0;
                            let th = tc_th & 0x0F;
                            if p + 16 > body.len() {
                                return Err(BlpError::new("jpeg_truncated_dht_counts"));
                            }
                            let mut counts = [0u8; 16];
                            counts.copy_from_slice(&body[p..p + 16]);
                            p += 16;
                            let symbols_len: usize = counts.iter().map(|&c| c as usize).sum();
                            if p + symbols_len > body.len() {
                                return Err(BlpError::new("jpeg_truncated_dht_symbols"));
                            }
                            let symbols = body[p..p + symbols_len].to_vec();
                            p += symbols_len;
                            if (th as usize) < 4 {
                                if class_dc {
                                    dht_dc[th as usize] = Some((counts, symbols));
                                } else {
                                    dht_ac[th as usize] = Some((counts, symbols));
                                }
                            }
                        }
                    }
                    0xDD => {
                        // DRI
                        if body.len() != 2 {
                            return Err(BlpError::new("jpeg_bad_dri"));
                        }
                        dri.interval = Some(be_u16(body)?);
                    }
                    0xC0 => {
                        // SOF0 baseline
                        if body.len() < 6 {
                            return Err(BlpError::new("jpeg_truncated_sof0"));
                        }
                        let precision = body[0];
                        let _height = be_u16(&body[1..3])?; // ignored here
                        let _width = be_u16(&body[3..5])?; // ignored here
                        let nf = body[5] as usize;
                        let mut comps = Vec::with_capacity(nf);
                        let mut p = 6;
                        for _ in 0..nf {
                            if p + 3 > body.len() {
                                return Err(BlpError::new("jpeg_truncated_sof0_comps"));
                            }
                            let id = body[p];
                            let hv = body[p + 1];
                            let h = hv >> 4;
                            let v = hv & 0x0F;
                            let tq = body[p + 2];
                            p += 3;
                            comps.push(SofComp { id, h, v, tq });
                        }
                        sof0 = Some(Sof0Template { precision, comps });
                    }
                    0xE0..=0xEF => {
                        app_segments.push(AppSegment { marker: m, payload: body.to_vec() });
                    }
                    0xDA => {
                        // SOS
                        if body.len() < 6 {
                            return Err(BlpError::new("jpeg_truncated_sos"));
                        }
                        let ns = body[0] as usize;
                        let mut comps = Vec::with_capacity(ns);
                        let mut p = 1;
                        for _ in 0..ns {
                            if p + 2 > body.len() {
                                return Err(BlpError::new("jpeg_truncated_sos_comps"));
                            }
                            let id = body[p];
                            let tdta = body[p + 1];
                            let td = tdta >> 4;
                            let ta = tdta & 0x0F;
                            p += 2;
                            comps.push(SosComp { id, td, ta });
                        }
                        if p + 3 > body.len() {
                            return Err(BlpError::new("jpeg_truncated_sos_tail"));
                        }
                        let ss = body[p];
                        let se = body[p + 1];
                        let ahal = body[p + 2];
                        let ah = ahal >> 4;
                        let al = ahal & 0x0F;

                        // Baseline constraints
                        if !(ss == 0 && se == 63 && ah == 0 && al == 0) {
                            return Err(BlpError::new("jpeg_non_baseline_sos"));
                        }

                        sos = Some(SosTemplate { comps, ss, se, ah, al });
                        // After SOS the stream enters entropy-coded data; plan is complete.
                        break;
                    }
                    _ => { /* ignore APPx/COM and other non-critical markers */ }
                }
                i = seg_end;
            }
        }
    }

    let sof0 = sof0.ok_or_else(|| BlpError::new("jpeg_missing_sof0"))?;
    let sos = sos.ok_or_else(|| BlpError::new("jpeg_missing_sos"))?;

    // Collect DQT/DHT actually referenced (minimal set).
    let mut used_dqts = Vec::<DqtTable>::new();
    for c in &sof0.comps {
        let tq = c.tq as usize;
        let Some(vals) = dqt_map[tq] else {
            return Err(BlpError::new("jpeg_missing_dqt"));
        };
        if !used_dqts.iter().any(|t| t.id == c.tq) {
            used_dqts.push(DqtTable { id: c.tq, vals });
        }
    }

    let mut used_dhts = Vec::<DhtTable>::new();
    for s in &sos.comps {
        // DC
        if let Some((counts, symbols)) = &dht_dc[s.td as usize] {
            if !used_dhts
                .iter()
                .any(|t| t.class_dc && t.id == s.td)
            {
                used_dhts.push(DhtTable { class_dc: true, id: s.td, counts_16: *counts, symbols: symbols.clone() });
            }
        } else {
            return Err(BlpError::new("jpeg_missing_dht"));
        }
        // AC
        if let Some((counts, symbols)) = &dht_ac[s.ta as usize] {
            if !used_dhts
                .iter()
                .any(|t| !t.class_dc && t.id == s.ta)
            {
                used_dhts.push(DhtTable { class_dc: false, id: s.ta, counts_16: *counts, symbols: symbols.clone() });
            }
        } else {
            return Err(BlpError::new("jpeg_missing_dht"));
        }
    }

    let plan = JpegPlan { dqt: used_dqts, dht: used_dhts, dri, app_segments, sof0, sos };
    plan.sanity()?;
    Ok(plan)
}
