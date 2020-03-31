use crate::NOTEFIELD_SIZE;
use crate::{Fraction, Measure, NoteData, NoteType, StructureData};

fn value(fraction: Fraction) -> f64 {
    f64::from(*fraction.numer()) / f64::from(*fraction.denom())
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Rectangle {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TimingData<T>
where
    T: TimingInfo,
{
    pub notes: [TimingColumn<T>; NOTEFIELD_SIZE],
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TimingColumn<T>
where
    T: TimingInfo,
{
    pub notes: Vec<T>,
}

pub trait TimingInfo: Copy {}

pub trait LayoutInfo {
    fn from_layout(time: i64, sprite: Rectangle, note: NoteType) -> Self;
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct GameplayInfo(pub i64, pub Rectangle, pub NoteType);

impl TimingInfo for GameplayInfo {}

impl LayoutInfo for GameplayInfo {
    fn from_layout(time: i64, sprite: Rectangle, note: NoteType) -> Self {
        Self(time, sprite, note)
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CalcInfo(pub i64, pub NoteType);

impl Rectangle {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
}

impl TimingInfo for CalcInfo {}

impl LayoutInfo for CalcInfo {
    fn from_layout(time: i64, _sprite: Rectangle, note: NoteType) -> Self {
        Self(time, note)
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Judgement {
    Hit(i64),
    Miss,
    Hold(bool), //true for OK, false for NG
    Mine(bool),
}

impl TimingInfo for Judgement {}

impl Judgement {
    pub fn wife(self, ts: f64) -> f64 {
        match self {
            Self::Hit(maxms) => {
                let avedeviation = 95.0 * ts;
                let mut y =
                    1.0 - 2.0_f64.powf(-(maxms * maxms) as f64 / (avedeviation * avedeviation));
                y *= y;
                (10.0) * (1.0 - y) - 8.0
            }
            Self::Miss => -8.0,
            Self::Hold(ok) => {
                if ok {
                    0.0
                } else {
                    -6.0
                }
            }
            Self::Mine(hit) => {
                if hit {
                    -8.0
                } else {
                    0.0
                }
            }
        }
    }
    pub fn max_points(self) -> f64 {
        match self {
            Self::Hit(_) | Self::Miss => 2.0,
            Self::Hold(_) | Self::Mine(_) => 0.0,
        }
    }
}

impl<T> TimingColumn<T>
where
    T: TimingInfo,
{
    pub fn add(&mut self, offset: T) {
        self.notes.push(offset);
    }
    pub fn new() -> Self {
        Self { notes: Vec::new() }
    }
}

impl<T> TimingData<T>
where
    T: TimingInfo + LayoutInfo,
{
    pub fn from_notedata<U>(data: &NoteData, sprite_finder: U, rate: f64) -> Vec<Self>
    where
        U: Fn(usize, f64, Fraction, NoteType, usize) -> Rectangle,
    {
        data.charts
            .iter()
            .map(|chart| Self::from_chartdata::<U>(chart, &data.structure, &sprite_finder, rate))
            .collect()
    }
    pub fn from_chartdata<U>(
        data: &[Measure],
        structure: &StructureData,
        sprite_finder: &U,
        rate: f64,
    ) -> Self
    where
        U: Fn(usize, f64, Fraction, NoteType, usize) -> Rectangle,
    {
        let offset = structure.offset.unwrap_or_default() * 1000.0;
        let mut bpms: Vec<_> = structure
            .bpms
            .iter()
            .map(|beat_pair| (beat_pair, 0.0))
            .collect();
        match bpms.get_mut(0) {
            Some(bpm) => bpm.1 = offset,
            None => return Self::new(),
        };
        for i in 1..bpms.len() {
            bpms[i].1 = bpms[i - 1].1
                + ((f64::from(bpms[i].0.beat - bpms[i - 1].0.beat)
                    + value(bpms[i].0.sub_beat - bpms[i - 1].0.sub_beat))
                    * 240_000.0
                    / bpms[i - 1].0.value);
        }
        let mut bpms = bpms.into_iter();
        let mut current_bpm = bpms.next().unwrap();
        let mut next_bpm = bpms.next();
        let mut output: [TimingColumn<T>; NOTEFIELD_SIZE] =
            array_init::array_init(|_| TimingColumn::new());
        for (measure_index, measure) in data.iter().enumerate() {
            for (row, inner_time) in measure.iter() {
                while let Some(bpm) = next_bpm {
                    if measure_index as i32 > bpm.0.beat
                        || (measure_index as i32 == bpm.0.beat
                            && bpm.0.sub_beat <= inner_time.fract())
                    {
                        current_bpm = bpm;
                        next_bpm = bpms.next();
                    } else {
                        break;
                    }
                }
                let row_time = (current_bpm.1
                    + 240_000.0
                        * ((measure_index - current_bpm.0.beat as usize) as f64
                            + value(inner_time - current_bpm.0.sub_beat))
                        / current_bpm.0.value)
                    / rate;
                for note in row.iter() {
                    let sprite =
                        sprite_finder(measure_index, 0.0, *inner_time, note.note_type, note.column);
                    //This if let can hide errors in the parser or .sm file
                    // An else clause should be added where errors are handled
                    if let Some(column) = output.get_mut(note.column) {
                        column.add(T::from_layout(row_time as i64, sprite, note.note_type));
                    }
                }
            }
        }
        Self { notes: output }
    }
}

impl<T> TimingData<T>
where
    T: TimingInfo,
{
    pub fn new() -> Self {
        Self {
            notes: array_init::array_init(|_| TimingColumn::new()),
        }
    }
}

//Unused functions here will be utilized when a results screen is added
impl TimingData<Judgement> {
    pub fn _max_points(&self) -> f64 {
        self.notes.iter().map(TimingColumn::max_points).sum()
    }
    pub fn _current_points(&self, ts: f64) -> f64 {
        self.notes.iter().map(|x| x.current_points(ts)).sum()
    }
    pub fn _calculate_score(&self, ts: f64) -> f64 {
        self._current_points(ts) / self._max_points()
    }
}

impl TimingColumn<Judgement> {
    pub fn max_points(&self) -> f64 {
        self.notes.iter().map(|x| x.max_points()).sum()
    }
    pub fn current_points(&self, ts: f64) -> f64 {
        self.notes.iter().map(|x| x.wife(ts)).sum()
    }
    pub fn _calculate_score(&self, ts: f64) -> f64 {
        self.current_points(ts) / self.max_points()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wife_symmetry() {
        for offset in 0..180 {
            let early = Judgement::Hit(-offset);
            let late = Judgement::Hit(offset);
            assert_eq!(early.wife(1.0), late.wife(1.0));
        }
    }
    #[test]
    fn wife_peak() {
        assert_eq!(Judgement::Hit(0).wife(1.0), 2.0);
        assert_eq!(Judgement::Hit(0).wife(0.5), 2.0);
        assert_eq!(Judgement::Hit(0).wife(2.0), 2.0);
    }
    #[test]
    fn wife_decreasing() {
        for offset in 0..179 {
            assert!(Judgement::Hit(offset).wife(1.0) > Judgement::Hit(offset + 1).wife(1.0));
            assert!(Judgement::Hit(offset).wife(0.5) > Judgement::Hit(offset + 1).wife(0.5));
            assert!(Judgement::Hit(offset).wife(2.0) > Judgement::Hit(offset + 1).wife(2.0));
        }
    }
}
