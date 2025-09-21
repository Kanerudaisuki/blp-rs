//! JPEG rebuild types: everything needed to deterministically reconstruct headers.
//!
//! This module defines compact data structures for:
//! - Quantization tables (DQT),
//! - Huffman tables (DHT),
//! - Optional restart interval (DRI),
//! - Frame template (SOF0) **without** width/height,
//! - Scan template (SOS),
//! - A combined `JpegPlan` used to rebuild headers for different mip sizes.
//!
//! Notes:
//! - All structures derive `PartialEq, Eq` so plans can be compared across mips.
//! - `JpegPlan::check_compatible_with` verifies two plans are identical in all
//!   reconstruction-relevant aspects (ignoring width/height since they are
//!   not stored in the template and are set per-mip later).

use crate::err::error::BlpError;

/// One quantization table (8-bit, 64 values).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DqtTable {
    pub id: u8,         // 0..3
    pub vals: [u8; 64], // zig-zag order as in the segment payload
}

/// One Huffman table (DC or AC).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DhtTable {
    pub class_dc: bool, // true = DC, false = AC
    pub id: u8,         // 0..3
    /// 16 code-length counts followed by symbols (raw, canonical as in JPEG).
    pub counts_16: [u8; 16],
    pub symbols: Vec<u8>,
}

/// Optional restart interval (MCUs).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Dri {
    pub interval: Option<u16>, // None => no DRI written
}

/// SOF0 component descriptor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SofComp {
    pub id: u8, // component id as written by encoder
    pub h: u8,  // sampling factor H (1..4)
    pub v: u8,  // sampling factor V (1..4)
    pub tq: u8, // quant table selector (0..3)
}

/// SOS component selector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SosComp {
    pub id: u8, // matches SOF0 component id
    pub td: u8, // DC Huffman table id (0..3)
    pub ta: u8, // AC Huffman table id (0..3)
}

/// Frame template (SOF0) without width/height.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sof0Template {
    pub precision: u8,       // usually 8
    pub comps: Vec<SofComp>, // 3 (BGR/YCC) or 4 (CMYK)
}

/// Scan template (SOS) — baseline single-scan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SosTemplate {
    pub comps: Vec<SosComp>, // order must match encoder’s SOS
    pub ss: u8,              // spectral start (0 for baseline)
    pub se: u8,              // spectral end   (63 for baseline)
    pub ah: u8,              // successive approx high (0 baseline)
    pub al: u8,              // successive approx low  (0 baseline)
}

/// Stored APP (application) marker payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppSegment {
    pub marker: u8, // 0xE0..=0xEF
    pub payload: Vec<u8>,
}

/// Custom APP marker used to stash JPEG plan metadata inside the shared header.
pub const BLP_PLAN_APP_MARKER: u8 = 0xEF;
const BLP_PLAN_MAGIC: &[u8; 8] = b"BLPPLAN\0";
const BLP_PLAN_VERSION: u8 = 0;

/// Lightweight plan template reconstructed from the embedded APP payload.
///
/// DQT/DHT are part of the common header, so we only need SOF0/SOS metadata here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanTemplate {
    pub sof0: Sof0Template,
    pub sos: SosTemplate,
}

/// Plan to rebuild headers/scans for all mips.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JpegPlan {
    /// All quantization tables used by the encoder.
    pub dqt: Vec<DqtTable>,
    /// All Huffman tables referenced by the scan.
    pub dht: Vec<DhtTable>,
    /// Optional restart interval.
    pub dri: Dri,
    /// Any APPx segments we want to preserve (e.g. Adobe color transforms).
    pub app_segments: Vec<AppSegment>,
    /// SOF0/SOS templates (no sizes inside).
    pub sof0: Sof0Template,
    pub sos: SosTemplate,
}

impl JpegPlan {
    /// Basic internal sanity checks (precision and component linkage).
    pub fn sanity(&self) -> Result<(), BlpError> {
        if self.sof0.precision != 8 {
            return Err(BlpError::new("jpeg_precision_not_8"));
        }
        if self.sof0.comps.is_empty() || self.sos.comps.is_empty() {
            return Err(BlpError::new("jpeg_no_components"));
        }
        // Each SOS comp id must exist in SOF0.
        for s in &self.sos.comps {
            if !self
                .sof0
                .comps
                .iter()
                .any(|c| c.id == s.id)
            {
                return Err(BlpError::new("jpeg_sos_id_not_in_sof0"));
            }
        }
        Ok(())
    }

    /// Compare two plans and return detailed reason on mismatch.
    pub fn check_compatible_with(&self, other: &JpegPlan) -> Result<(), BlpError> {
        // precision
        if self.sof0.precision != other.sof0.precision {
            eprintln!("[plan] mismatch: precision {} vs {}", self.sof0.precision, other.sof0.precision);
            return Err(BlpError::new("jpeg_plan_mismatch"));
        }

        // SOF0 comps
        if self.sof0.comps.len() != other.sof0.comps.len() {
            eprintln!("[plan] mismatch: SOF0 comps len {} vs {}", self.sof0.comps.len(), other.sof0.comps.len());
            return Err(BlpError::new("jpeg_plan_mismatch"));
        }
        for (i, (a, b)) in self
            .sof0
            .comps
            .iter()
            .zip(other.sof0.comps.iter())
            .enumerate()
        {
            if a.id != b.id || a.h != b.h || a.v != b.v || a.tq != b.tq {
                eprintln!("[plan] mismatch: SOF0 comp#{i}: id/h/v/tq = ({},{},{},{}) vs ({},{},{},{})", a.id, a.h, a.v, a.tq, b.id, b.h, b.v, b.tq);
                return Err(BlpError::new("jpeg_plan_mismatch"));
            }
        }

        // SOS
        if self.sos.comps.len() != other.sos.comps.len() {
            eprintln!("[plan] mismatch: SOS comps len {} vs {}", self.sos.comps.len(), other.sos.comps.len());
            return Err(BlpError::new("jpeg_plan_mismatch"));
        }
        for (i, (a, b)) in self
            .sos
            .comps
            .iter()
            .zip(other.sos.comps.iter())
            .enumerate()
        {
            if a.id != b.id || a.td != b.td || a.ta != b.ta {
                eprintln!("[plan] mismatch: SOS comp#{i}: id/td/ta = ({},{},{}) vs ({},{},{})", a.id, a.td, a.ta, b.id, b.td, b.ta);
                return Err(BlpError::new("jpeg_plan_mismatch"));
            }
        }
        if self.sos.ss != other.sos.ss || self.sos.se != other.sos.se || self.sos.ah != other.sos.ah || self.sos.al != other.sos.al {
            eprintln!("[plan] mismatch: SOS params ss/se/ah/al = ({},{},{},{}) vs ({},{},{},{})", self.sos.ss, self.sos.se, self.sos.ah, self.sos.al, other.sos.ss, other.sos.se, other.sos.ah, other.sos.al);
            return Err(BlpError::new("jpeg_plan_mismatch"));
        }

        // APP segments
        if self.app_segments != other.app_segments {
            eprintln!("[plan] mismatch: APP segments differ ({} vs {})", self.app_segments.len(), other.app_segments.len());
            return Err(BlpError::new("jpeg_plan_mismatch"));
        }

        // DRI
        if self.dri.interval != other.dri.interval {
            eprintln!("[plan] mismatch: DRI interval {:?} vs {:?}", self.dri.interval, other.dri.interval);
            return Err(BlpError::new("jpeg_plan_mismatch"));
        }

        // DQT
        let mut a_q = self.dqt.clone();
        a_q.sort_by_key(|t| t.id);
        let mut b_q = other.dqt.clone();
        b_q.sort_by_key(|t| t.id);
        if a_q.len() != b_q.len() {
            eprintln!("[plan] mismatch: DQT len {} vs {}", a_q.len(), b_q.len());
            return Err(BlpError::new("jpeg_plan_mismatch"));
        }
        for (i, (a, b)) in a_q.iter().zip(b_q.iter()).enumerate() {
            if a.id != b.id || a.vals != b.vals {
                eprintln!("[plan] mismatch: DQT table#{i} id {} vs {}, or vals differ", a.id, b.id);
                return Err(BlpError::new("jpeg_plan_mismatch"));
            }
        }

        // DHT
        let mut a_h = self.dht.clone();
        a_h.sort_by_key(|t| (t.class_dc, t.id));
        let mut b_h = other.dht.clone();
        b_h.sort_by_key(|t| (t.class_dc, t.id));
        if a_h.len() != b_h.len() {
            eprintln!("[plan] mismatch: DHT len {} vs {}", a_h.len(), b_h.len());
            return Err(BlpError::new("jpeg_plan_mismatch"));
        }
        for (i, (a, b)) in a_h.iter().zip(b_h.iter()).enumerate() {
            if a.class_dc != b.class_dc || a.id != b.id || a.counts_16 != b.counts_16 || a.symbols != b.symbols {
                eprintln!("[plan] mismatch: DHT table#{i} (class_dc,id)=({},{}) vs ({},{}), or payload differs", a.class_dc, a.id, b.class_dc, b.id);
                return Err(BlpError::new("jpeg_plan_mismatch"));
            }
        }

        Ok(())
    }
}

pub fn encode_plan_app_payload(sof0: &Sof0Template, sos: &SosTemplate) -> Vec<u8> {
    let mut payload = Vec::with_capacity(BLP_PLAN_MAGIC.len() + 1 + 1 + sof0.comps.len() * 4 + 1 + sos.comps.len() * 3 + 4);
    payload.extend_from_slice(BLP_PLAN_MAGIC);
    payload.push(BLP_PLAN_VERSION);
    payload.push(sof0.precision);
    payload.push(sof0.comps.len() as u8);
    for c in &sof0.comps {
        payload.push(c.id);
        payload.push(c.h);
        payload.push(c.v);
        payload.push(c.tq);
    }
    payload.push(sos.comps.len() as u8);
    for c in &sos.comps {
        payload.push(c.id);
        payload.push(c.td);
        payload.push(c.ta);
    }
    payload.push(sos.ss);
    payload.push(sos.se);
    payload.push(sos.ah);
    payload.push(sos.al);
    payload
}

pub fn decode_plan_app_payload(payload: &[u8]) -> Result<PlanTemplate, BlpError> {
    if payload.len() < BLP_PLAN_MAGIC.len() + 1 + 1 + 1 + 4 {
        return Err(BlpError::new("jpeg_plan_app_too_short"));
    }
    if payload[..BLP_PLAN_MAGIC.len()] != *BLP_PLAN_MAGIC {
        return Err(BlpError::new("jpeg_plan_app_bad_magic"));
    }
    let mut idx = BLP_PLAN_MAGIC.len();
    let version = payload[idx];
    idx += 1;
    if version != BLP_PLAN_VERSION {
        return Err(BlpError::new("jpeg_plan_app_bad_version"));
    }
    let precision = payload[idx];
    idx += 1;
    let comp_count = payload[idx] as usize;
    idx += 1;
    if payload.len() < idx + comp_count * 4 + 1 {
        return Err(BlpError::new("jpeg_plan_app_truncated_sof0"));
    }
    let mut sof_comps = Vec::with_capacity(comp_count);
    for _ in 0..comp_count {
        let id = payload[idx];
        let h = payload[idx + 1];
        let v = payload[idx + 2];
        let tq = payload[idx + 3];
        idx += 4;
        sof_comps.push(SofComp { id, h, v, tq });
    }
    if idx >= payload.len() {
        return Err(BlpError::new("jpeg_plan_app_truncated_sos"));
    }
    let sos_count = payload[idx] as usize;
    idx += 1;
    if payload.len() < idx + sos_count * 3 + 4 {
        return Err(BlpError::new("jpeg_plan_app_truncated_sos"));
    }
    let mut sos_comps = Vec::with_capacity(sos_count);
    for _ in 0..sos_count {
        let id = payload[idx];
        let td = payload[idx + 1];
        let ta = payload[idx + 2];
        idx += 3;
        sos_comps.push(SosComp { id, td, ta });
    }
    let ss = payload[idx];
    let se = payload[idx + 1];
    let ah = payload[idx + 2];
    let al = payload[idx + 3];

    Ok(PlanTemplate { sof0: Sof0Template { precision, comps: sof_comps }, sos: SosTemplate { comps: sos_comps, ss, se, ah, al } })
}

pub fn extract_plan_template_from_common_header(common: &[u8]) -> Result<Option<PlanTemplate>, BlpError> {
    if common.len() < 2 || common[0] != 0xFF || common[1] != 0xD8 {
        return Err(BlpError::new("jpeg_plan_common_bad_soi"));
    }
    let mut i = 2usize;
    while i + 1 < common.len() {
        if common[i] != 0xFF {
            return Err(BlpError::new("jpeg_plan_common_bad_marker"));
        }
        while i < common.len() && common[i] == 0xFF {
            i += 1;
        }
        if i >= common.len() {
            break;
        }
        let marker = common[i];
        i += 1;
        match marker {
            0x01 | 0xD0..=0xD7 => {
                continue;
            }
            _ => {
                if i + 2 > common.len() {
                    return Err(BlpError::new("jpeg_plan_common_truncated_len"));
                }
                let seg_len = u16::from_be_bytes([common[i], common[i + 1]]) as usize;
                if i + seg_len > common.len() {
                    return Err(BlpError::new("jpeg_plan_common_truncated_segment"));
                }
                if marker == BLP_PLAN_APP_MARKER {
                    let payload = &common[i + 2..i + seg_len];
                    let tpl = decode_plan_app_payload(payload)?;
                    return Ok(Some(tpl));
                }
                i += seg_len;
            }
        }
    }
    Ok(None)
}

/// Simple slices (for each mip) to avoid reparsing when writing full JPEG.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JpegSlices {
    /// Bytes from start of file up to and including the SOS header.
    pub head_len: usize,
    /// Entropy-coded scan length (excludes the trailing EOI marker).
    pub scan_len: usize,
}
