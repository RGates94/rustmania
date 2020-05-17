use crate::NOTEFIELD_SIZE;
use ggez::{
    error::GameResult,
    graphics::{self, Rect, WrapMode},
};
use notedata::timingdata::Rectangle;
use notedata::{
    timingdata::{GameplayInfo, Judgement},
    NoteType,
};
use serde_derive::Deserialize;
use std::{fs::File, io::Read, path::Path};

#[derive(Clone, PartialEq, Debug)]
pub struct NoteLayout {
    pub sprites: NoteSprites,
    pub column_positions: [i64; NOTEFIELD_SIZE],
    pub column_rotations: [f32; NOTEFIELD_SIZE],
    pub receptor_height: i64,
    pub judgment_position: [f32; 2],
    pub scroll_speed: f32,
}

#[derive(PartialEq, Clone, Debug)]
pub struct NoteSkin {
    pub sprites: NoteSprites,
    pub column_positions: [i64; NOTEFIELD_SIZE],
    pub column_rotations: [f32; NOTEFIELD_SIZE],
}

#[derive(PartialEq, Clone, Debug)]
pub struct NoteSprites {
    pub arrows: graphics::Image,
    pub receptor: graphics::Image,
    pub judgment: graphics::Image,
    pub hold_body: graphics::Image,
    pub hold_end: graphics::Image,
    pub mine: graphics::Image,
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub struct PlayerOptions {
    notefield_position: i64,
    receptor_height: i64,
    scroll_speed: f32,
    is_reverse: bool,
    judgment_position: (f32, f32),
}

fn to_ggez(rect: Rectangle) -> ggez::graphics::Rect {
    let Rectangle { x, y, w, h } = rect;
    Rect::new(x, y, w, h)
}

impl NoteLayout {
    pub fn new(skin: &NoteSkin, screen_height: i64, player_options: PlayerOptions) -> Self {
        let NoteSkin {
            sprites,
            mut column_positions,
            mut column_rotations,
        } = skin.clone();
        let PlayerOptions {
            notefield_position,
            mut receptor_height,
            mut scroll_speed,
            is_reverse,
            mut judgment_position,
        } = player_options;
        column_positions
            .iter_mut()
            .for_each(|x| *x += notefield_position);
        column_rotations.iter_mut().for_each(|x| *x *= 6.28 / 360.0);
        judgment_position.0 += notefield_position as f32;
        if is_reverse {
            receptor_height = screen_height - receptor_height;
            judgment_position.1 = screen_height as f32 - judgment_position.1;
            scroll_speed *= -1.0;
        }
        let judgment_position = [judgment_position.0, judgment_position.1];
        Self {
            sprites,
            column_positions,
            column_rotations,
            receptor_height,
            judgment_position,
            scroll_speed,
        }
    }
    pub fn delta_to_position(&self, delta: i64) -> i64 {
        (delta as f32 * self.scroll_speed) as i64 + self.receptor_height
    }
    pub fn delta_to_offset(&self, delta: i64) -> f32 {
        delta as f32 * self.scroll_speed
    }
    pub fn add_note(
        &self,
        column: usize,
        column_data: &[GameplayInfo],
        batches: &mut Vec<graphics::spritebatch::SpriteBatch>,
    ) {
        let GameplayInfo(position, coords, note_type) = match column_data.get(0) {
            Some(val) => *val,
            None => return,
        };
        let position = self.delta_to_position(position);
        let batch_index = match note_type {
            NoteType::Tap | NoteType::Roll | NoteType::Lift | NoteType::Fake => 2,
            NoteType::Hold => {
                if let Some(GameplayInfo(end, _, _)) = column_data.get(1) {
                    batches[1].add(
                        graphics::DrawParam::new()
                            .src(Rect::new(
                                0.0,
                                0.0,
                                1.0,
                                (position - self.delta_to_position(*end)) as f32 / 64.0
                                    + if self.scroll_speed > 0.0 { 0.5 } else { -0.5 },
                            ))
                            .dest([self.column_positions[column] as f32, position as f32])
                            .rotation(if note_type == NoteType::Tap {
                                self.column_rotations[column]
                            } else {
                                0.0
                            })
                            .offset([0.5, 1.0]),
                    );
                };
                2
            }
            NoteType::Mine => 3,
            NoteType::HoldEnd => 0,
        };
        batches[batch_index].add(
            graphics::DrawParam::new()
                .src(to_ggez(coords))
                .dest([self.column_positions[column] as f32, position as f32])
                .rotation(if note_type == NoteType::Tap {
                    self.column_rotations[column]
                } else {
                    0.0
                })
                .offset([0.5, 0.5])
                .scale(if batch_index == 0 && self.scroll_speed > 0.0 {
                    [1.0, -1.0]
                } else {
                    [1.0, 1.0]
                }),
        );
    }
    pub fn add_column_of_notes(
        &self,
        column: &[GameplayInfo],
        column_index: usize,
        batches: &mut Vec<graphics::spritebatch::SpriteBatch>,
    ) {
        for index in 0..column.len() {
            self.add_note(column_index, &column[index..], batches);
        }
    }
    pub fn draw_receptors(&self, ctx: &mut ggez::Context) -> Result<(), ggez::GameError> {
        for (index, &column_position) in self.column_positions.iter().enumerate() {
            graphics::draw(
                ctx,
                &self.sprites.receptor,
                graphics::DrawParam::new()
                    .dest([column_position as f32, self.receptor_height as f32])
                    .rotation(self.column_rotations[index])
                    .offset([0.5, 0.5]),
            )?;
        }
        Ok(())
    }
    pub fn add_hold(
        &self,
        ctx: &mut ggez::Context,
        column_index: usize,
        delta: i64,
    ) -> Result<(), ggez::GameError> {
        let is_reverse = if self.scroll_speed > 0.0 { 1.0 } else { -1.0 };
        graphics::draw(
            ctx,
            &self.sprites.hold_end,
            graphics::DrawParam::new()
                .dest([
                    self.column_positions[column_index] as f32,
                    self.delta_to_position(delta) as f32,
                ])
                .offset([0.5, 0.5]),
        )?;
        graphics::draw(
            ctx,
            &self.sprites.hold_body,
            graphics::DrawParam::new()
                .src(graphics::Rect::new(0.0, 0.0, 1.0, {
                    let dist = self.delta_to_offset(delta) / 64.0 * is_reverse;
                    if dist < 0.0 {
                        0.0
                    } else {
                        dist
                    }
                }))
                .dest([
                    self.column_positions[column_index] as f32,
                    (self.delta_to_position(delta)) as f32,
                ])
                .offset([0.5, 0.0])
                .scale([1.0, -is_reverse]),
        )?;
        Ok(())
    }
    //this will likely be the method to draw receptors in the future, but it is not currently in use
    pub fn _add_receptors(
        &self,
        batch: &mut graphics::spritebatch::SpriteBatch,
    ) -> Result<(), ggez::GameError> {
        for &column_position in &self.column_positions {
            batch.add(
                graphics::DrawParam::new()
                    .dest([column_position as f32, self.receptor_height as f32]),
            );
        }
        Ok(())
    }
    fn select_judgment(&self, judge: Judgement) -> Option<graphics::DrawParam> {
        let src = match judge {
            Judgement::Hit(-22..=22) => graphics::Rect::new(0.0, 0.0, 1.0, 0.1666),
            Judgement::Hit(-45..=45) => graphics::Rect::new(0.0, 0.1666, 1.0, 0.1666),
            Judgement::Hit(-90..=90) => graphics::Rect::new(0.0, 0.3333, 1.0, 0.1666),
            Judgement::Hit(-135..=135) => graphics::Rect::new(0.0, 0.5, 1.0, 0.1666),
            Judgement::Hit(-180..=180) => graphics::Rect::new(0.0, 0.6666, 1.0, 0.1666),
            Judgement::Hit(out_of_range) => {
                println!();
                panic!("Hit was registered outside the normal execution window with offset of {} milliseconds: Aborting",out_of_range)
            }
            Judgement::Miss => graphics::Rect::new(0.0, 0.8333, 1.0, 1.666),
            Judgement::Hold(_) | Judgement::Mine(_) => {
                return None;
            }
        };
        Some(
            graphics::DrawParam::new()
                .src(src)
                .dest(self.judgment_position),
        )
    }
    pub fn draw_judgment(
        &self,
        ctx: &mut ggez::Context,
        judge: Judgement,
    ) -> Result<(), ggez::GameError> {
        if let Some(param) = self.select_judgment(judge) {
            graphics::draw(ctx, &self.sprites.judgment, param)?;
        }
        Ok(())
    }
}

#[derive(Deserialize)]
struct NoteSkinInfo {
    arrows: String,
    receptor: String,
    judgment: String,
    hold_body: String,
    hold_head: String,
    mine: String,
    column_positions: [i64; NOTEFIELD_SIZE],
    column_rotations: [f32; NOTEFIELD_SIZE],
}

impl NoteSkin {
    pub fn new(path: &Path, context: &mut ggez::Context) -> Option<Self> {
        let mut config_file = match File::open(path.join("config.toml")) {
            Ok(file) => file,
            Err(_) => return None,
        };
        let mut config_string = String::new();
        match config_file.read_to_string(&mut config_string) {
            Ok(_) => {}
            Err(_) => return None,
        };
        let NoteSkinInfo {
            arrows,
            receptor,
            judgment,
            hold_body,
            hold_head,
            mine,
            column_positions,
            column_rotations,
        } = match toml::from_str(&config_string) {
            Ok(skin) => skin,
            Err(_) => return None,
        };
        if let (
            Ok(arrows),
            Ok(receptor),
            Ok(judgment),
            Ok(mut hold_body),
            Ok(hold_head),
            Ok(mine),
        ) = (
            image_from_subdirectory(context, path, &arrows),
            image_from_subdirectory(context, path, &receptor),
            image_from_subdirectory(context, path, &judgment),
            image_from_subdirectory(context, path, &hold_body),
            image_from_subdirectory(context, path, &hold_head),
            image_from_subdirectory(context, path, &mine),
        ) {
            hold_body.set_wrap(WrapMode::Tile, WrapMode::Tile);
            let sprites = NoteSprites {
                arrows,
                receptor,
                judgment,
                hold_body,
                hold_end: hold_head,
                mine,
            };
            Some(Self {
                sprites,
                column_positions,
                column_rotations,
            })
        } else {
            None
        }
    }
}

fn image_from_subdirectory(
    context: &mut ggez::Context,
    path: &Path,
    extension: &str,
) -> GameResult<graphics::Image> {
    graphics::Image::new(context, Path::new("/").join(path).join(extension))
}

impl PlayerOptions {
    pub fn new(
        notefield_position: i64,
        receptor_height: i64,
        scroll_speed: f32,
        is_reverse: bool,
        judgment_position: (f32, f32),
    ) -> Self {
        Self {
            notefield_position,
            receptor_height,
            scroll_speed,
            is_reverse,
            judgment_position,
        }
    }
}
