use std::ops::Range;

use crate::fl::GSDFile;

pub struct ConfigurationData {
    step: u64,
    dimensions: u8,
    box_: [f32; 6],
}

#[derive(Default)]
pub struct ParticleData<'a> {
    position: Option<&'a [[f32; 3]]>,
    orientation: Option<&'a [[f32; 4]]>,
    typeid: Option<&'a [u32]>,
    mass: Option<&'a [f32]>,
    charge: Option<&'a [f32]>,
    diameter: Option<&'a [f32]>,
    body: Option<&'a [i32]>,
    moment_inertia: Option<&'a [[f32; 3]]>,
    velocity: Option<&'a [[f32; 3]]>,
    angmom: Option<&'a [[f32; 4]]>,
    image: Option<&'a [[f32; 3]]>,
    types: Option<&'a [&'a str]>,
}

pub struct BondData<'a, const M: usize> {
    n: u32,
    types: &'a [&'a str],
    typeid: &'a [u32],
    group: &'a [[u32; M]],
}

pub struct ConstraintData<'a> {
    n: u32,
    value: &'a [f32],
    group: &'a [[f32; 2]],
}

pub struct Snapshot<'a> {
    configuration: ConfigurationData,
    particles: ParticleData<'a>,
    bonds: BondData<'a, 2>,
    angles: BondData<'a, 3>,
    dihedrals: BondData<'a, 4>,
    impropers: BondData<'a, 4>,
    constraints: ConstraintData<'a>,
    pairs: BondData<'a, 2>,
}

pub struct HOOMDTrajectoryView<'a> {
    trajectory: &'a HOOMDTrajectory,
    indices: (Range<usize>, usize)
}

pub struct HOOMDTrajectory {
    file: GSDFile,
    initial_frame: Option<usize>,
}

impl HOOMDTrajectory {
    pub fn new(file: GSDFile) -> Self {
        Self { file, initial_frame: None }
    }
}
