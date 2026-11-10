//! `RwFrame`, `RpAtomic`, `RpClump`.

use std::sync::Arc;

use sa_formats::rw::{RwStream, dff, id};

use crate::geometry::Geometry;
use crate::matrix::RwMatrix;
use crate::{Error, Result, ids, lower_name, u32_at};

/// `RwFrame`: modelling matrix + hierarchy (parent index instead of child/next/root pointers).
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    /// `RwFrame::modelling`.
    pub modelling: RwMatrix,
    /// Parent frame index (`None` for the root).
    pub parent: Option<usize>,
    /// Lower-case node name (node name plugin 0x253F2FE), empty when absent.
    pub name: String,
}

/// `RpAtomic`.
#[derive(Debug, Clone, PartialEq)]
pub struct Atomic {
    /// Index into [`Clump::frames`].
    pub frame: usize,
    /// `RpAtomic::geometry` (geometries can be shared between atomics).
    pub geometry: Arc<Geometry>,
    /// `rpATOMIC*` flags (collision test 0x1, render 0x4).
    pub flags: u32,
    /// Custom pipeline id from the pipeline set plugin (0x253F2F3), if any.
    pub pipeline_id: Option<u32>,
}

impl Atomic {
    pub const FLAG_RENDER: u32 = 0x4;

    pub fn renders(&self) -> bool {
        self.flags & Self::FLAG_RENDER != 0
    }
}

/// `RpClump`.
#[derive(Debug, Clone, PartialEq)]
pub struct Clump {
    pub frames: Vec<Frame>,
    pub atomics: Vec<Atomic>,
}

impl Clump {
    pub fn from_dff(c: &dff::Clump) -> Result<Clump> {
        let frames: Vec<Frame> = c
            .frame_list
            .frames
            .iter()
            .enumerate()
            .map(|(i, f)| Frame {
                modelling: RwMatrix {
                    right: f.rotation[0],
                    up: f.rotation[1],
                    at: f.rotation[2],
                    pos: f.position,
                    flags: f.matrix_flags,
                },
                parent: (f.parent >= 0 && (f.parent as usize) < i).then_some(f.parent as usize),
                name: c
                    .frame_list
                    .extensions
                    .get(i)
                    .and_then(|e| e.find(ids::NODENAMEPLUGIN))
                    .and_then(|ch| ch.data())
                    .map(lower_name)
                    .unwrap_or_default(),
            })
            .collect();
        let geometries: Vec<Arc<Geometry>> = c
            .geometry_list
            .geometries
            .iter()
            .map(|g| Geometry::from_dff(g).map(Arc::new))
            .collect::<Result<_>>()?;
        let mut atomics = Vec::with_capacity(c.atomics.len());
        for a in &c.atomics {
            let geometry = match &a.geometry {
                Some(g) => Arc::new(Geometry::from_dff(g)?),
                None => geometries
                    .get(a.geometry_index as usize)
                    .cloned()
                    .ok_or_else(|| Error::Invalid(format!("atomic geometry index {} out of range", a.geometry_index)))?,
            };
            let frame = a.frame_index as usize;
            if frame >= frames.len() {
                return Err(Error::Invalid(format!("atomic frame index {frame} out of range")));
            }
            let pipeline_id = a.extension.find(ids::PIPELINEPLUGIN).and_then(|c| c.data()).and_then(|b| u32_at(b, 0));
            atomics.push(Atomic { frame, geometry, flags: a.flags, pipeline_id });
        }
        Ok(Clump { frames, atomics })
    }

    /// The LTM of frame `i` relative to the clump's root frame (`RwFrameGetLTM` with the root at
    /// the identity).
    pub fn frame_ltm(&self, i: usize) -> RwMatrix {
        let f = &self.frames[i];
        match f.parent {
            Some(p) => f.modelling.mul(&self.frame_ltm(p)),
            None => RwMatrix::IDENTITY,
        }
    }
}

/// Reads the first clump of a DFF stream (`RpClumpStreamRead`).
pub fn read_clump(data: &[u8]) -> Result<Clump> {
    let stream = RwStream::parse(data)?;
    let c = stream.chunks.iter().find(|c| c.ty == id::CLUMP).ok_or(Error::Missing("clump"))?;
    Clump::from_dff(&dff::Clump::from_chunk(c)?)
}
