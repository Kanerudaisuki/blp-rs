//! Header re-builder: emit minimal, deterministic baseline header
//! from a parsed `JpegPlan`.
//!
//! Common header = SOI + DQT* + DHT* + [DRI?]
//! Per-mip tail  = SOF0(width,height) + SOS
//! Full JPEG     = common + per-mip tail + scan + EOI

use crate::encode::blp::jpeg::types::*;
use crate::err::error::BlpError;
use byteorder::{BigEndian, WriteBytesExt};

#[inline]
fn push_marker(out: &mut Vec<u8>, marker: u8) {
    out.push(0xFF);
    out.push(marker);
}

#[inline]
fn push_segment(out: &mut Vec<u8>, marker: u8, payload: &[u8]) -> Result<(), BlpError> {
    push_marker(out, marker);
    // length = 2 (len bytes) + payload
    let len = (payload.len() + 2) as u16;
    out.write_u16::<BigEndian>(len)?;
    out.extend_from_slice(payload);
    Ok(())
}

#[inline]
fn build_dqt_payload(table: &DqtTable) -> Vec<u8> {
    // Pq=0 (8-bit), Tq=id
    let mut v = Vec::with_capacity(1 + 64);
    v.push((0 << 4) | (table.id & 0x0F));
    v.extend_from_slice(&table.vals);
    v
}

#[inline]
fn build_dht_payload(table: &DhtTable) -> Vec<u8> {
    // Tc (high nibble) = 0 for DC, 1 for AC; Th (low nibble)
    let mut v = Vec::with_capacity(1 + 16 + table.symbols.len());
    let tc = if table.class_dc { 0 } else { 1 };
    v.push(((tc & 0x0F) << 4) | (table.id & 0x0F));
    v.extend_from_slice(&table.counts_16);
    v.extend_from_slice(&table.symbols);
    v
}

/// Build minimal common header: SOI + [DQT*] + [DHT*] + [DRI?].
/// SOF0/SOS are **not** included here (они будут добавлены на уровне мипа).
pub fn build_common_header(plan: &JpegPlan) -> Result<Vec<u8>, BlpError> {
    // Санити на всякий случай.
    plan.sanity()?;

    let mut out = Vec::new();

    // SOI
    out.extend_from_slice(&[0xFF, 0xD8]);

    // Optional APP markers (e.g. Adobe APP14 for CMYK color alignment)
    for seg in &plan.app_segments {
        push_segment(&mut out, seg.marker, &seg.payload)?;
    }
    if !plan
        .app_segments
        .iter()
        .any(|s| s.marker == BLP_PLAN_APP_MARKER)
    {
        let payload = encode_plan_app_payload(&plan.sof0, &plan.sos);
        push_segment(&mut out, BLP_PLAN_APP_MARKER, &payload)?;
    }

    // DQT — отдельно по таблице, детерминированно по id
    let mut dqts = plan.dqt.clone();
    dqts.sort_by_key(|t| t.id);
    for t in &dqts {
        let payload = build_dqt_payload(t);
        push_segment(&mut out, 0xDB, &payload)?;
    }

    // DHT — сначала DC, затем AC; сортировка по (class,id)
    let mut dhts = plan.dht.clone();
    dhts.sort_by_key(|t| (if t.class_dc { 0u8 } else { 1u8 }, t.id));
    for t in &dhts {
        let payload = build_dht_payload(t);
        push_segment(&mut out, 0xC4, &payload)?;
    }

    // DRI (если есть)
    if let Some(n) = plan.dri.interval {
        let payload = (n as u16).to_be_bytes();
        push_segment(&mut out, 0xDD, &payload)?;
    }

    Ok(out)
}

/// Build per-mip SOF0 (with width/height) + SOS from templates.
pub fn build_sof0_sos(plan: &JpegPlan, width: u16, height: u16) -> Result<Vec<u8>, BlpError> {
    plan.sanity()?;
    build_sof0_sos_from_templates(&plan.sof0, &plan.sos, width, height)
}

pub fn build_sof0_sos_from_templates(sof0: &Sof0Template, sos: &SosTemplate, width: u16, height: u16) -> Result<Vec<u8>, BlpError> {
    if width == 0 || height == 0 {
        return Err(BlpError::new("jpeg_zero_dim"));
    }

    let mut out = Vec::new();

    // SOF0 (baseline)
    {
        let nf = sof0.comps.len() as u8;
        // 8 байт заголовка + 3 байта на компонент
        let mut payload = Vec::with_capacity(8 + 3 * (nf as usize));
        payload.push(sof0.precision);
        payload.extend_from_slice(&height.to_be_bytes());
        payload.extend_from_slice(&width.to_be_bytes());
        payload.push(nf);
        for c in &sof0.comps {
            payload.push(c.id);
            payload.push((c.h << 4) | (c.v & 0x0F));
            payload.push(c.tq);
        }
        push_segment(&mut out, 0xC0, &payload)?;
    }

    // SOS (baseline, single scan)
    {
        let ns = sos.comps.len() as u8;
        let mut payload = Vec::with_capacity(6 + 2 * (ns as usize));
        payload.push(ns);
        for c in &sos.comps {
            payload.push(c.id);
            payload.push(((c.td & 0x0F) << 4) | (c.ta & 0x0F));
        }
        payload.push(sos.ss);
        payload.push(sos.se);
        payload.push(((sos.ah & 0x0F) << 4) | (sos.al & 0x0F));
        push_segment(&mut out, 0xDA, &payload)?;
    }

    Ok(out)
}

/// Build a complete JPEG in-place for a mip: `common + sof0+sos + scan + EOI`.
pub fn assemble_full_jpeg(common_header: &[u8], sof0_sos: &[u8], scan: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(common_header.len() + sof0_sos.len() + scan.len() + 2);
    out.extend_from_slice(common_header);
    out.extend_from_slice(sof0_sos);
    out.extend_from_slice(scan);
    out.extend_from_slice(&[0xFF, 0xD9]); // EOI
    out
}
