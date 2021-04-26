use std::{error::Error, path::Path, sync::{Arc, Mutex}};
use std::rc::Rc;
use std::fs::File;
use std::io::{BufReader, prelude::*};
use clap::{App, Arg};
use rand::{Rng, SeedableRng, StdRng};

use rodio::buffer::SamplesBuffer;
use synthrs::wave;
use wgpu::SwapChainError;
use winit::window::WindowBuilder;
use winit::event::VirtualKeyCode;
use winit_input_helper::WinitInputHelper;

use engine3d::{graphics::{gpu::{GAME_HEIGHT, GAME_WIDTH}, graphics::{GraphicalDisplay, GraphicsMethod}, sprite::UpdateSprite}, logic::{collision::{gather_contacts, killed, restitute}, types::*}};
use engine3d::{audio::audio::{Note, SoundChannels, generate_samples}};
use engine3d::graphics::tiles::*;
use engine3d::graphics::animation::*;
use engine3d::logic::state::*;
use engine3d::graphics::maps::{get_start_map, get_empty_map, get_maps};


// use engine3d::collision::*;
// Imagine a Resources struct (we'll call it AssetDB or Assets in the future)
// which wraps all accesses to textures, sounds, animations, etc.
use engine3d::graphics::resources::*;

const WIDTH: usize = 480;
const HEIGHT: usize = 320;
const MAX_SPEED: i32 = 4;
const X_SPEED: i32 = 3;

fn main() {
    let args = App::new("Game 1")
                          .version("1.0")
                          .author("Nette Mashewske and Max Rose")
                          .about("Avoid the spikes!")
                          .arg(
                                Arg::with_name("backend")
                                    .takes_value(true)
                                    .default_value("opengl")
                                    .value_names(&["opengl", "vulkan"])
                                    .help("Set the graphics backend. Options are CPU, OpenGl, or Vulkan / DX12")
                                    .long_help("Sets the graphics backend. Options are CPU, OpenGl, or Vulkan / DX12. 
                                                    The CPU renderer is based on Pixel using bitblt while the other options are based on wgpu.
                                                    The CPU renderer does not permit screen resizing.")
                                    .next_line_help(true)
                            )
                            .get_matches();
    let render_type = match args.value_of("backend") {
        Some("opengl") => GraphicsMethod::OpenGL,
        Some("vulkan") => GraphicsMethod::WGPUDefault,
        Some(invalid) => panic!("Invalid backend type: {}", invalid),
        None => panic!("No backend set!")
    };
    let window_builder = match render_type {
        _ => {
            WindowBuilder::new()
                .with_title("Game 1")
                .with_resizable(true)
                .with_maximized(true)
        } 
    };
    let start_map = get_start_map();
    let empty_map = get_empty_map();
    let maps = get_maps();
    // Here's our resources...
    let rsrc = Resources::new();
    let tileset = Rc::new(Tileset::new(
        vec![
            Tile{solid:false, triangle:false},
            Tile{solid:true, triangle: false},
            Tile{solid:true, triangle: false},
            Tile{solid:true, triangle: false},
            Tile{solid:true, triangle: false},
            Tile{solid:true, triangle: false},
            Tile{solid:true, triangle: false},
            Tile{solid:true, triangle: false},
            Tile{solid:true, triangle: false},
            Tile{solid:true, triangle: false},
            Tile{solid:false, triangle: true}
        ],
        &rsrc.load_texture(Path::new("resources/tileset.png"))
    ));
    // Here's our game rules (the engine doesn't know about these)
    let tilemaps: Vec<Tilemap> = get_starting_map(&start_map, &empty_map, &  tileset, 4);
    let mut levels:Vec<Level> = vec![
        (
            // The map
            tilemaps,
            // Initial entities on level start
            vec![
                (EntityType::Player, 2, 13),
            ],
            maps,
            start_map,
            empty_map
        )
    ];
    // Load animations and textures
    let ent_anims = EntityAnims::new();
    let player_tex = rsrc.load_texture(Path::new("resources/blob.png"));
    let player_anim = ent_anims.standing.start();
    let text = &rsrc.load_texture(Path::new("resources/text.png"));

    // Work with saved data
    let score = 0;
    let mut rng = rand::thread_rng();
    let seed = rng.gen::<u64>();
    
    let mut rng: StdRng= SeedableRng::seed_from_u64(seed);
    for _i in 0..3 {
        add_maps(& mut levels[0], &mut rng);
    }

    // And here's our game state, which is just stuff that changes.
    // We'll say an entity is a type, a position, a velocity, a size, a texture, and an animation state.
    // State here will stitch them all together.
    let state = StateType::Menu(GameState{
        // Every entity has a position, a size, a texture, and animation state.
        // Assume entity 0 is the player
        types: vec![
            // In a real example we'd provide nicer accessors than this
            levels[0].1[0].0,
        ],
        positions: vec![
            Vec2i(
                levels[0].1[0].1 * 16,
                levels[0].1[0].2 * 16,
            )
        ],
        velocities: vec![Vec2i(X_SPEED,0)],
        sizes: vec![(16,16)],
        // Could be texture handles instead, let's talk about that in two weeks
        textures: vec![Rc::clone(&player_tex),
                       Rc::clone(&text)],
        anim_state: vec![player_anim],
        ent_states: vec![EntityState::Falling],
        // Current level
        level: 0,
        // Camera position
        camera: Vec2i(0, 0),
        inputs: Inputs::new(),
        menu_entry: 0,
        entity_anims: ent_anims,
        sound_channels: Arc::new(Mutex::new(SoundChannels::new())),
        score,
        seed,
        rng
    });
    engine3d::run(WIDTH, HEIGHT, window_builder,
            rsrc, levels, state, render_type, init, draw, update_game);
}

fn init(_resources:&Resources, levels: &mut Vec<Level>, screen_type: &mut GraphicalDisplay, state: &StateType) -> Result<(), Box<dyn Error>> {
    match screen_type {
        GraphicalDisplay::Gpu(gpu_state) => {
            match state {
                StateType::Menu(game_state) => {
                    for texture in &game_state.textures {
                        gpu_state.load_sprite(texture.clone());
                    }
                    gpu_state.clear_color = wgpu::Color{r:75.0 / 255.0,g:105.0 / 255.0,b:47.0 / 255.0,a:1.0};
                    gpu_state.load_tilemap(&mut levels[game_state.level].0[0])
                },
                _ => {panic!("Initialized to invalid state!")},
            }
        }
    }
}

fn draw_menu(_resources:&Resources,_levelss: &Vec<Level>, state: &GameState,
        screen_type: &mut GraphicalDisplay, _frame:usize) -> Result<(), SwapChainError> {
    match screen_type {
        GraphicalDisplay::Gpu(gpu_state) => {
            let _width = WIDTH as i32;
            let _height = HEIGHT as i32;
            assert!(state.menu_entry < 2);
            gpu_state.camera.pos = [0.0, 0.0];
            gpu_state.update();
            let mut commands = Vec::new();
            commands.push(gpu_state.update_sprite_instances(0, &state.positions[0..0], &state.anim_state[0..0])?);
            match state.menu_entry {
                0 => {
                    commands.push(gpu_state.update_sprite_instances(1, &[Vec2i(0, 64), Vec2i(0, 90)], &[
                        Rc::new(Animation::new(vec![(Rect{x: 0, y: 16, w:64, h: 16}, 1 as usize)], true)).start(),
                        Rc::new(Animation::new(vec![(Rect{x: 0, y: 32, w:64, h: 16}, 1 as usize)], true)).start(),
                    ])?);
                },
                1 =>  {
                    commands.push(gpu_state.update_sprite_instances(1, &[Vec2i(0, 64), Vec2i(0, 90)], &[
                        Rc::new(Animation::new(vec![(Rect{x: 0, y: 0, w:64, h: 16}, 1 as usize)], true)).start(),
                        Rc::new(Animation::new(vec![(Rect{x: 0, y: 48, w:64, h: 16}, 1 as usize)], true)).start(),
                    ])?);
                },
                _ => {},
            }
            let (command, frame) = gpu_state.clear_screen()?;
            commands.push(command);
            commands.push(gpu_state.render_sprites(&frame)?);
            gpu_state.queue.submit(commands);
            Ok(())
        }
    }
    
}

fn draw_game_over(_resources:&Resources,_levelss: &Vec<Level>, state: &GameState,
        screen_type: &mut GraphicalDisplay, _frame:usize) -> Result<(), SwapChainError> {
    match screen_type {
        GraphicalDisplay::Gpu(gpu_state) => {
            gpu_state.camera.pos = [0.0, 0.0];
            gpu_state.update();
            let mut commands = Vec::new();
            commands.push(gpu_state.update_sprite_instances(0, &state.positions[0..1], &state.anim_state[0..1])?);
            commands.push(gpu_state.update_sprite_instances(1, &[Vec2i(0, 64), Vec2i(0, 90)], &[
                Rc::new(Animation::new(vec![(Rect{x: 0, y: 64, w:64, h: 16}, 1 as usize)], true)).start(),
                Rc::new(Animation::new(vec![(Rect{x: 0, y: 80, w:64, h: 16}, 1 as usize)], true)).start(),
            ])?);
            let (command, frame) = gpu_state.clear_screen()?;
            commands.push(command);
            commands.push(gpu_state.render_sprites(&frame)?);
            gpu_state.queue.submit(commands);
            Ok(())
        }
    }
}

fn draw_game(_resources:&Resources, levels: &Vec<Level>, state: &GameState,
        screen_type: &mut GraphicalDisplay, _frame:usize) -> Result<(), SwapChainError> {
    match screen_type {
        GraphicalDisplay::Gpu(gpu_state) => {
            let player_pos = state.positions[0];
            let camera_pos = gpu_state.camera.pos;
            gpu_state.camera.pos[0] = camera_pos[0].max(player_pos.0 as f32 - GAME_WIDTH / 3.0).min(player_pos.0 as f32 + GAME_WIDTH / 3.0);
            gpu_state.camera.pos[1] = camera_pos[1].max(player_pos.1 as f32 - GAME_HEIGHT / 3.0).min(player_pos.1 as f32 + GAME_HEIGHT / 3.0);
            gpu_state.update();
            let mut commands = Vec::new();
            commands.push(gpu_state.update_sprite_instances(0, &state.positions[0..1], &state.anim_state[0..1])?);
            // Workaround for text not disappearing. Moves text out of view instead
            commands.push(gpu_state.update_sprite_instances(1, &[Vec2i(i32::MIN / 2, i32::MIN / 2), Vec2i(i32::MIN / 2, i32::MIN / 2)], &[
                Rc::new(Animation::new(vec![(Rect{x: 0, y: 64, w:64, h: 16}, 1 as usize)], true)).start(),
                Rc::new(Animation::new(vec![(Rect{x: 0, y: 80, w:64, h: 16}, 1 as usize)], true)).start(),
            ])?);
            let (encoder, texture) = gpu_state.clear_screen()?;
            commands.push(encoder);
            levels[state.level].0.iter().for_each(|map| {
                commands.push(gpu_state.update_tilemap_instances(map).unwrap());
                commands.push(gpu_state.render_tiles(&texture).unwrap());
            });
            commands.push(gpu_state.render_tiles(&texture)?);
            commands.push(gpu_state.render_sprites(&texture)?);
            gpu_state.queue.submit(commands);
            Ok(())
        }
    }
}

fn draw(resources:&Resources, levels: &Vec<Level>, state: &StateType,
        screen_type: &mut GraphicalDisplay, frame:usize) -> Result<(), SwapChainError> {
    match state {
        StateType::Menu(game_state) => draw_menu(resources, levels, game_state, screen_type, frame),
        StateType::Playing(game_state) => draw_game(resources, levels, game_state, screen_type, frame),
        StateType::GameOver(game_state) => draw_game_over(resources, levels, game_state, screen_type, frame),
    }
}

fn update_game(levels: &mut Vec<Level>, game_state: &mut StateType, input: &WinitInputHelper,_framee: usize) -> bool {
    match game_state {
        StateType::GameOver(state) => {
            return match input.key_pressed(VirtualKeyCode::Space) {
                true => {
                    println!("Final Score: {}", state.score);
                    true
                },
                false => false
            }
        }
        StateType::Menu(state) => {
            let mut score = 0;
            if input.key_pressed(VirtualKeyCode::L){
                let loaded_game = load_game();
                match loaded_game {
                    Ok((sc, sd, pos)) => {
                        score = sc;
                        state.seed = sd;
                        state.positions[0] = pos;
                        println!("Previous game loaded successfully, current score {}", score);
                    },
                    Err(_) => {}
                }
                state.positions[0].0 = ((score + 3) * 16 * 16 - (16 * 16) / 2)  as i32;
                state.rng = SeedableRng::seed_from_u64(state.seed);
                let tilemaps: Vec<Tilemap> = get_starting_map(&get_start_map(), &get_empty_map(), & levels[state.level].0[0].tileset, 4);
                levels[state.level].0 = tilemaps;
                for _i in 0..3 {
                    add_maps(& mut levels[state.level], & mut state.rng);
                }
                
                for _i in 0..score {
                    update_tilemaps(& mut levels[state.level], &mut state.rng, state.score);
                }
                state.score = score;
            }
            state.inputs.up = input.key_pressed(VirtualKeyCode::Up);
            state.inputs.down = input.key_pressed(VirtualKeyCode::Down);
            state.inputs.space = input.key_pressed(VirtualKeyCode::Space);
            if state.inputs.space && state.menu_entry == 1 {
                return true;
            }
            else if state.inputs.space && state.menu_entry == 0 {
                *game_state = StateType::Playing(state.clone());
                return false;
            }
            else if state.inputs.up {
                
                state.menu_entry = (state.menu_entry + 1) % 2;
                let channels = state.sound_channels.lock().unwrap();
                channels.stream_handle.play_raw(select_sound()).unwrap();
            }
            else if state.inputs.down {
                state.menu_entry = match state.menu_entry {
                    1 => 0,
                    _ => 1
                };
                let channels = state.sound_channels.lock().unwrap();
                channels.stream_handle.play_raw(select_sound()).unwrap();
            }
        }
        StateType::Playing(state) => {

            // Save the game
            if input.key_pressed(VirtualKeyCode::S){
                let position = state.positions[0];
                match save_game(state.score, state.seed,position){
                    Ok(_) => println!("Game saved successfuly!"),
                    Err(_) => println!("Error Saving game")
                }
            }
            state.anim_state.iter_mut().for_each(|x| x.tick());
            // Player control goes here
            state.inputs.right = input.key_pressed(VirtualKeyCode::Right) ||
                input.key_held(VirtualKeyCode::Right);
            state.inputs.left = input.key_pressed(VirtualKeyCode::Left) ||
                input.key_held(VirtualKeyCode::Left);
            state.inputs.space = input.key_pressed(VirtualKeyCode::Space) ||
                input.key_held(VirtualKeyCode::Space);
            state.inputs.esc = input.key_pressed(VirtualKeyCode::Escape);
            if state.inputs.esc {
                state.menu_entry = 0;
                *game_state = StateType::Menu(state.clone());
                return false;
            }
            let player_rect = Rect{
                x:state.positions[0].0,
                y:state.positions[0].1,
                w:state.sizes[0].0 as u16,
                h:state.sizes[0].1 as u16
            };
            update_velocity(state.ent_states.get_mut(0).unwrap(), player_rect, state.velocities.get_mut(0).unwrap(),
                            &levels[state.level].0,
                             state.anim_state.get_mut(0).unwrap(), &state.inputs, &state.entity_anims, state.sound_channels.clone());
    
            // Update all entities' positions
            for (posn, vel) in state.positions.iter_mut().zip(state.velocities.iter()) {
                posn.0 += vel.0;
                posn.1 += vel.1;
            }

            
            // Detect collisions: Convert positions and sizes to collision bodies, generate contacts
            let mut contacts = Vec::new();
            let tilemaps = &levels.get(state.level).unwrap().0;
            let mut game_over = false;
            state.positions.iter().zip(0..state.positions.len())
                                .map(|(pos, id)| (Rect{x: pos.0 + 7, y:pos.1, w:11, h:16}, id))
                                .for_each(|(rect, id)| {
                                    if gather_contacts(tilemaps, &rect, id, &mut contacts){
                                        game_over = true;
                                    }
                                });
            // Handle collisions: Apply restitution impulses.
            if restitute(&mut state.positions, &mut state.ent_states, &mut state.velocities, &state.sizes, &mut contacts,
                 &mut state.anim_state, &state.entity_anims){
                     game_over = true;
                 }
            // Update game rules: What happens when the player touches things?  When enemies touch walls?  Etc.
            let updated_collisions: Vec<Rect> = state.positions.iter()
                        .map(|pos| Rect{x: pos.0 + 7, y:pos.1, w:11, h:16})
                        .collect();
            if killed(updated_collisions.get(0).unwrap(), &updated_collisions[1..]) {
                game_over = true;
            }
            if game_over{
                let channels = state.sound_channels.lock().unwrap();
                channels.stream_handle.play_raw(game_over_sound()).unwrap();
            }
            if game_over{
                state.positions.iter_mut().for_each(|pos| {
                    *pos = Vec2i(i32::MIN / 2, i32::MIN / 2);
                });
                *game_state = StateType::GameOver(state.clone());
            }
            else {
                // Maybe scroll the camera or change level
                let mut new_cam_pos = *state.positions.get(0).unwrap();
                new_cam_pos.0 -= WIDTH as i32 / 2;
                new_cam_pos.1 -= HEIGHT as i32 / 2;
                state.camera = new_cam_pos;

                // Check if we should add more maps
                if state.positions[0].0 > levels[state.level].0[5].position.0{
                    state.score = update_tilemaps(&mut levels[state.level], &mut state.rng, state.score);
                    println!("Score: {}", state.score);
                }
            }
        }
    }
    
    false
}

fn update_velocity(state: &mut EntityState, entity: Rect, vel: &mut Vec2i, level: &Vec<Tilemap>, anim: &mut AnimationState, inputs: &Inputs, ent_anims: &EntityAnims, sound_channels: Arc<Mutex<SoundChannels>>) {

    match state {
        EntityState::Jumping if vel.1 < 0 => {
            vel.1 = (vel.1 + 1).min(MAX_SPEED);
        },
        EntityState::Jumping if vel.1 > 0 => {
            vel.1 = (vel.1 - 1).max(-MAX_SPEED);
            *state = EntityState::Falling;
            *anim = ent_anims.falling.start();
        },
        EntityState::Landing if anim.done() => {
            if vel.0 != 0 {
                let channels = sound_channels.lock().unwrap();
                channels.stream_handle.play_raw(land_sound()).unwrap();
                *state = EntityState::Running;
                *anim = ent_anims.running.start();
            }
            else {
                *state = EntityState::Standing;
                *anim = ent_anims.standing.start();
            }
        },
        EntityState::StartRun if anim.done() 
                                    && vel.0 != 0 => {
            *state = EntityState::Running;
            *anim = ent_anims.running.start();
        },
        EntityState::Standing if !inputs.space && vel.0 != 0 => {
            *state = EntityState::StartRun;
            *anim = ent_anims.start_run.start();
        },
        EntityState::Running if vel.0 == 0 => {
            *state = EntityState::Standing;
            *anim = ent_anims.standing.start();
        },
        EntityState::StartRun if vel.0 == 0 => {
            *state = EntityState::Standing;
            *anim = ent_anims.standing.start();
        },
        _ if inputs.space && vel.1 == 0 => {  
            vel.1 = -5;
            *state = EntityState::Jumping;
            *anim = ent_anims.jumping.start();
            if !airborn(Vec2i(entity.x, entity.y + entity.h as i32), level) || 
            !airborn(Vec2i(entity.x + entity.w as i32, entity.y + entity.h as i32), level){
                let channels = sound_channels.lock().unwrap();
                channels.stream_handle.play_raw(jump_sound()).unwrap();
            }

        },
        _ => {
            // Make sure the character is airborn then move down
            if airborn(Vec2i(entity.x, entity.y + entity.h as i32), level) || 
            airborn(Vec2i(entity.x + entity.w as i32, entity.y + entity.h as i32), level){
                vel.1 = (vel.1 + 1).min(MAX_SPEED);
            }
        },
    }
}

// Checks right below the two bottom edges to see if character is airborn
fn airborn(Vec2i(x, y): Vec2i, level: &Vec<Tilemap>) -> bool{
    for map in level.iter() {
        if map.contains(Vec2i(x, y)) && map.tile_at(Vec2i(x, y + 1)).solid {
            return false
        }
    }
    true
}

// Creates a starting map of base tiles size long
fn get_starting_map(start_map: &Vec<usize>, empty_map: &Vec<usize>, tileset: &Rc<Tileset>, size: i32) -> Vec<Tilemap> {
    let mut tilemaps = vec![];

    tilemaps.push(Tilemap::new(
        Vec2i(0, 0),
        // Map size
        (16, 16),
        &tileset,
        // Tile grid
        start_map.clone(), 
    ));

    for i in 1..size {
        tilemaps.push(
            Tilemap::new(
            Vec2i(i * 16 * 16,0),
            // Map size
            (16, 16),
            &tileset,
            // Tile grid
            empty_map.clone(), 
        )
        )
    }
    tilemaps
}

// Adds two maps to the current list
fn add_maps(level: &mut Level, rng: &mut StdRng){
    let next_map = rng.choose(&level.2).unwrap();
    level.0.push(Tilemap::new(
        Vec2i(level.0.last().unwrap().position.0 + 16*16, 0),
        // Map size
        (16, 16),
        &level.0[0].tileset,
        // Tile grid
        level.4.clone(), 
        )
    );

    level.0.push(Tilemap::new(
            Vec2i(level.0.last().unwrap().position.0 + 16*16, 0),
            // Map size
            (16, 16),
            &level.0[0].tileset,
            // Tile grid
            next_map.clone(), 
        )
    );
}

// Removes the first two maps then adds two more
fn update_tilemaps(level: &mut Level, rng: &mut StdRng, score: usize) -> usize{

    // let mut rng = thread_rng();
    let next_map = rng.choose(&level.2).unwrap();
    level.0.remove(0);
    level.0.remove(0);
    
    level.0.push(Tilemap::new(
        Vec2i(level.0.last().unwrap().position.0 + 16*16, 0),
        // Map size
        (16, 16),
        &level.0[0].tileset,
        // Tile grid
        level.4.clone(), 
        )
    );

    level.0.push(Tilemap::new(
            Vec2i(level.0.last().unwrap().position.0 + 16*16, 0),
            // Map size
            (16, 16),
            &level.0[0].tileset,
            // Tile grid
            next_map.clone(), 
        )
    );
    score + 1
}

fn jump_sound() -> SamplesBuffer<f32> {
    let notes = vec![Note::new(0, 4, 1)];
    generate_samples(notes, 240.0, wave::organ)
}

fn land_sound() -> SamplesBuffer<f32> {
    let notes = vec![Note::new(6, 3, 1)];
    generate_samples(notes, 240.0, wave::organ)
}

fn game_over_sound() -> SamplesBuffer<f32> {
    let notes = vec![
        Note::new(0, 5, 2),
        Note::new(11, 4, 2),
        Note::new(10, 4, 2),
        Note::new(9, 4, 4),
    ];
    generate_samples(notes, 240.0, wave::organ)
}

fn select_sound() -> SamplesBuffer<f32> {
    let notes = vec![
        Note::new(0, 6, 1),
        Note::new(2, 6, 1),
    ];
    generate_samples(notes, 480.0, wave::organ)
}

// Writes seed and score to file
fn save_game(score: usize, seed: u64, Vec2i(x, y): Vec2i)-> std::io::Result<()>{
    let mut file = File::create("save_file.txt")?;
    file.write_all(format!("{}\n{}\n{}\n{}", score, seed, x, y).as_bytes())?;

    Ok(())
}

// Loads game information
fn load_game() -> std::io::Result<(usize, u64, Vec2i)>{
    let f = File::open("save_file.txt")?;
    let f = BufReader::new(f);
    let mut result_vec = vec![];
    for line in f.lines(){
        result_vec.push(line?);
    }

    let score = &result_vec[0];
    let seed = &result_vec[1];
    let x = &result_vec[2];
    let y = &result_vec[3];
    let score = score.parse::<usize>().unwrap();
    let seed = seed.parse::<u64>().unwrap();
    let x = x.parse().unwrap();
    let y = y.parse().unwrap();
    let pos = Vec2i(x, y);
    Ok((score, seed, pos))
}