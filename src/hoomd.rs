use std::ops::Range;
use ndarray::Slice;

use crate::fl::GSDFile;

#[derive(Default)]
pub struct ConfigurationData {
    step: u64,
    dimensions: u8,
    box_: [f32; 6],
}

#[derive(Default)]
pub struct ParticleData<'a> {
    n: u32,
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

#[derive(Default)]
pub struct BondData<'a, const M: usize> {
    n: u32,
    types: &'a [&'a str],
    typeid: &'a [u32],
    group: &'a [[u32; M]],
}

#[derive(Default)]
pub struct ConstraintData<'a> {
    n: u32,
    value: &'a [f32],
    group: &'a [[f32; 2]],
}

#[derive(Default)]
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

pub struct HOOMDTrajectoryIterator<'a> {
    trajectory: &'a HOOMDTrajectory<'a>,
    slice: (Range<usize>, usize),
}

impl<'a> HOOMDTrajectoryIterator<'a> {
    fn new(trajectory: &'a HOOMDTrajectory, range: Range<usize>, stride: usize) -> Self {
        assert!(stride > 0);
        Self {
            trajectory,
            slice: (range, stride),
        }
    }

    fn len(&self) -> usize {
        self.slice.0.len() / self.slice.1
    }
}

impl<'a> Iterator for HOOMDTrajectoryIterator<'a> {
    type Item = Snapshot<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.slice.0.start >= self.slice.0.end {
            return None;
        }

        let snapshot = self.trajectory.index(self.slice.0.start);
        self.slice.0.start += self.slice.1;
        Some(snapshot)
    }
}

pub struct HOOMDTrajectoryView<'a> {
    trajectory: &'a HOOMDTrajectory<'a>,
    slice: (Range<usize>, usize),
}

impl<'a> HOOMDTrajectoryView<'a> {
    fn index(&'a self, idx: usize) -> Snapshot<'a> {
        let idx = self.slice.0.start + idx * self.slice.1;
        if idx >= self.slice.0.end {
            panic!("index out of bounds");
        }
        let snap = self.trajectory._read_frame(idx);
        snap
    }

    fn view(&'a self, mut slice: (Range<usize>, usize)) -> HOOMDTrajectoryView<'a> {
        let items = self.slice.0.len() / self.slice.1;
        if slice.0.end > items {
            slice.0.end = items;
        }
        let slice = (
            slice.0.start * self.slice.1 + self.slice.0.start
                ..slice.0.end * self.slice.1 + self.slice.0.start,
            slice.1 * self.slice.1,
        );
        HOOMDTrajectoryView {
            trajectory: self.trajectory,
            slice,
        }
    }

    fn iter(&'a self) -> HOOMDTrajectoryIterator<'a> {
        self.into_iter()
    }

    fn len(&self) -> usize {
        self.slice.0.len() / self.slice.1
    }
}

impl<'a> IntoIterator for &'a HOOMDTrajectoryView<'a> {
    type Item = Snapshot<'a>;
    type IntoIter = HOOMDTrajectoryIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        let slice = self.slice.to_owned();
        HOOMDTrajectoryIterator {
            trajectory: self.trajectory,
            slice,
        }
    }
}

pub struct HOOMDTrajectory<'a> {
    file: GSDFile,
    initial_frame: Option<Snapshot<'a>>,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a> HOOMDTrajectory<'a> {
    pub fn new(file: GSDFile) -> Self {
        Self {
            file,
            initial_frame: None,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn try_new(file: GSDFile) -> Self {
        if file.mode() == "ab" {
            panic!("Append not yet supported!");
        }
        if file.schema() != "hoomd" {
            panic!("Not a HOOMD schema!");
        }

        let version = file.schema_version();
        if version < (2, 0) && version >= (1, 0) {
            panic!("Incompatible HOOMD schema version!")
        }

        Self::new(file)
    }

    fn _read_frame(&'a self, idx: usize) -> Snapshot<'a> {

        if idx > self.len() {
            panic!("Index out of bounds!");
        }

        if self.initial_frame.is_none() && idx != 0 {
            self._read_frame(0);
        }

        let step: usize = if self.file.chunk_exists(idx, "configuration/step") {
            self.file.read_chunk_flat(idx, "configuration/step").unwrap()[0]
        }
        else {
            self.file.read_chunk_flat(idx, "configuration/step").unwrap()[0]
        };



        Snapshot::default()
    }

    fn index(&'a self, idx: usize) -> Snapshot<'a> {
        let snap = self._read_frame(idx);
        snap
    }

    fn view(&'a self, slice: (Range<usize>, usize)) -> HOOMDTrajectoryView<'a> {
        HOOMDTrajectoryView {
            trajectory: self,
            slice,
        }
    }

    fn iter(&'a self) -> HOOMDTrajectoryIterator<'a> {
        self.into_iter()
    }

    fn len(&self) -> usize {
        self.file.nframes()
    }
}

impl<'a> IntoIterator for &'a HOOMDTrajectory<'a> {
    type Item = Snapshot<'a>;
    type IntoIter = HOOMDTrajectoryIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        let slice = (0..self.file.nframes(), 1);
        HOOMDTrajectoryIterator {
            trajectory: self,
            slice,
        }
    }
}
