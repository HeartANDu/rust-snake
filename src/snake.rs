use std::collections::{HashMap, VecDeque};
use std::ops::Mul;
use std::time::Duration;
use bevy::{
    prelude::*,
    sprite::collide_aabb::collide,
    text::Text2dBounds,
};
use rand::Rng;

const BLOCK_SIZE: Vec3 = Vec3::new(20.0, 20.0, 1.0);
const SCREEN_HEIGHT: f32 = 22.0;
const SCREEN_WIDTH: f32 = 40.0;

const SCORE_DELTA: usize = 100;
const SCOREBOARD_FONT_SIZE: f32 = 21.0;
const SCOREBOARD_PADDING: Val = Val::Px(10.0);

const MESSAGE_BOX_SIZE: Vec2 = Vec2::new(450.0, 200.0);
const MESSAGE_BOX_FONT_SIZE: f32 = 30.0;

const SNAKE_STARTING_LENGTH: i32 = 4;
const SNAKE_STARTING_POSITION: Position = Position::new(0.0, 0.0);
const SNAKE_STARTING_DIRECTION: Direction = Direction::Right;

const TIMER_STARTING_DURATION: f32 = 0.16;
const TIMER_SCALING_PERCENTAGE: f32 = 15.0;
const SCORE_DIFFICULTY_THRESHOLD: f32 = 500.0;

const WALL_COLOR: Color = Color::rgb(0.8, 0.8, 0.8);
const MOUSE_COLOR: Color = Color::rgb(1.0, 0.65, 0.34);
const SNAKE_COLOR: Color = Color::rgb(1.0, 1.0, 1.0);
const SCOREBOARD_COLOR: Color = Color::rgb(1.0, 1.0, 1.0);
const MESSAGE_BOX_BACKGROUND_COLOR: Color = Color::rgb(1.0, 1.0, 1.0);
const MESSAGE_BOX_TEXT_COLOR: Color = Color::rgb(0.0, 0.0, 0.0);
const BACKGROUND_COLOR: Color = Color::rgb(0.1, 0.1, 0.1);

const MAX_INPUT_QUEUE_LENGTH: usize = 2;

pub struct SnakeApp;

impl Plugin for SnakeApp {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(BACKGROUND_COLOR))
            .insert_resource(MoveTimer(Timer::from_seconds(TIMER_STARTING_DURATION, TimerMode::Repeating)))
            .insert_resource(Scoreboard { score: 0, difficulty: 0 })
            .add_state::<GameState>()
            .add_event::<SoundEvent>()
            .add_systems(Startup, (setup_once, setup))
            .add_systems(Update, (handle_state_input, play_sounds))
            .add_systems(Update, (
                update_scoreboard,
                update_difficulty,
                move_snake,
                check_collisions,
            ).run_if(in_state(GameState::Running)))
            .add_systems(OnEnter(GameState::Startup), spawn_message::<StartupMessage>)
            .add_systems(OnExit(GameState::Startup), despawn::<StartupMessage>)
            .add_systems(OnEnter(GameState::Paused), spawn_message::<PausedMessage>)
            .add_systems(OnExit(GameState::Paused), despawn::<PausedMessage>)
            .add_systems(OnEnter(GameState::GameOver), (spawn_message::<GameOverMessage>, game_over))
            .add_systems(OnExit(GameState::GameOver), (
                despawn::<GameOverMessage>,
                despawn::<GameComponents>,
                reset,
                setup,
            ))
        ;
    }
}

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
enum GameState {
    #[default]
    Startup,
    Running,
    Paused,
    GameOver,
}

#[derive(Component)]
struct GameComponents;

#[derive(Resource, Deref, DerefMut)]
struct MoveTimer(Timer);

#[derive(Component)]
struct Snake(u32);

#[derive(Bundle)]
struct SnakeBundle {
    block_bundle: BlockBundle,
    snake: Snake,
    direction: Direction,
    collider: Collider,
    game_component: GameComponents,
}

impl SnakeBundle {
    fn new(id: u32, block_bundle: BlockBundle, direction: Direction) -> SnakeBundle {
        SnakeBundle {
            block_bundle,
            snake: Snake(id),
            direction,
            collider: Collider,
            game_component: GameComponents,
        }
    }
}

#[derive(Component)]
struct Mouse;

#[derive(Bundle)]
struct MouseBundle {
    block_bundle: BlockBundle,
    mouse: Mouse,
    collider: Collider,
    game_component: GameComponents,
}

impl MouseBundle {
    fn new(block_size: Vec3) -> MouseBundle {
        let x_pos = SCREEN_WIDTH / 2.0 - 1.0;
        let y_pos = SCREEN_HEIGHT / 2.0 - 1.0;

        let mut rng = rand::thread_rng();

        MouseBundle {
            block_bundle: BlockBundle::new(
                MOUSE_COLOR,
                Position(Vec2::new(
                    rng.gen_range(-x_pos..=x_pos).round(),
                    rng.gen_range(-y_pos..=y_pos).round(),
                )),
                block_size,
            ),
            mouse: Mouse,
            collider: Collider,
            game_component: GameComponents,
        }
    }
}

#[derive(Bundle)]
struct BlockBundle {
    sprite_bundle: SpriteBundle,
    position: Position,
}

impl BlockBundle {
    fn new(color: Color, position: Position, block_size: Vec3) -> BlockBundle {
        BlockBundle {
            sprite_bundle: SpriteBundle {
                transform: Transform {
                    translation: Vec3::new(
                        position.x * block_size.x,
                        position.y * block_size.y,
                        0.0,
                    ),
                    scale: block_size,
                    ..default()
                },
                sprite: Sprite {
                    color,
                    ..default()
                },
                ..default()
            },
            position,
        }
    }
}

#[derive(Component, Clone)]
struct Id(i32);

#[derive(Component, Deref, DerefMut)]
struct Position(Vec2);

impl Position {
    const fn new(x: f32, y: f32) -> Position {
        Position(Vec2::new(x, y))
    }

    fn apply_vel(&mut self, velocity: &Velocity) {
        self.x += velocity.x;
        self.y += velocity.y;
    }

    fn translation(&self) -> Vec3 {
        Vec3::new(
            self.x * BLOCK_SIZE.x,
            self.y * BLOCK_SIZE.y,
            0.0,
        )
    }
}

#[derive(Component, Copy, Clone, PartialEq, Debug)]
enum Direction {
    Left,
    Right,
    Down,
    Up,
}

impl Direction {
    fn velocity(&self) -> Velocity {
        match self {
            Direction::Left => Velocity(Vec2::new(-1.0, 0.0)),
            Direction::Right => Velocity(Vec2::new(1.0, 0.0)),
            Direction::Down => Velocity(Vec2::new(0.0, -1.0)),
            Direction::Up => Velocity(Vec2::new(0.0, 1.0)),
        }
    }

    fn reverse(&self) -> Direction {
        match self {
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
            Direction::Down => Direction::Up,
            Direction::Up => Direction::Down,
        }
    }
}

#[derive(Component, Deref, DerefMut)]
struct Velocity(Vec2);

#[derive(Bundle)]
struct WallBundle {
    sprite_bundle: SpriteBundle,
    collider: Collider,
    game_component: GameComponents,
}

impl WallBundle {
    fn new(location: WallLocation, block_size: Vec3) -> WallBundle {
        WallBundle {
            sprite_bundle: SpriteBundle {
                transform: Transform {
                    translation: location.translation(block_size),
                    scale: location.scale(block_size),
                    ..default()
                },
                sprite: Sprite {
                    color: WALL_COLOR,
                    ..default()
                },
                ..default()
            },
            collider: Collider,
            game_component: GameComponents,
        }
    }
}

#[derive(Component)]
struct Collider;

enum WallLocation {
    Left,
    Right,
    Bottom,
    Top,
}

impl WallLocation {
    fn translation(&self, block_size: Vec3) -> Vec3 {
        let (start, end) = self.points();

        let x_pos = (start.x + end.x) / 2.0;
        let y_pos = (start.y + end.y) / 2.0;

        Vec3::new(x_pos, y_pos, 0.0).mul(block_size)
    }

    fn scale(&self, block_size: Vec3) -> Vec3 {
        let (start, end) = self.points();

        let dx = (start.x - end.x).abs() + 1.0;
        let dy = (start.y - end.y).abs() + 1.0;

        Vec3::new(dx, dy, 1.0).mul(block_size)
    }

    fn points(&self) -> (Vec2, Vec2) {
        let x_pos = SCREEN_WIDTH / 2.0;
        let y_pos = SCREEN_HEIGHT / 2.0;

        match self {
            WallLocation::Left => (Vec2::new(-x_pos, -y_pos), Vec2::new(-x_pos, y_pos)),
            WallLocation::Right => (Vec2::new(x_pos, y_pos), Vec2::new(x_pos, -y_pos)),
            WallLocation::Bottom => (Vec2::new(x_pos, -y_pos), Vec2::new(-x_pos, -y_pos)),
            WallLocation::Top => (Vec2::new(-x_pos, y_pos), Vec2::new(x_pos, y_pos)),
        }
    }
}

#[derive(Resource)]
struct Scoreboard {
    score: usize,
    difficulty: usize,
}

#[derive(Component)]
struct ScoreboardComponent;

#[derive(Resource)]
struct Sounds {
    sounds: HashMap<SoundType, Handle<AudioSource>>
}

impl Sounds {
    fn new() -> Sounds {
        Sounds { sounds: HashMap::new() }
    }

    fn add_sound(&mut self, sound_type: SoundType, source: Handle<AudioSource>) {
        self.sounds.insert(sound_type, source);
    }

    fn get_sound(&self, sound_type: &SoundType) -> Option<Handle<AudioSource>> {
        match self.sounds.get(sound_type) {
            None => None,
            Some(sound) => Some(sound.clone()),
        }
    }
}

#[derive(PartialEq, Eq, Hash)]
enum SoundType {
    Silence,
    Grow,
    DifficultyUp,
    Failure,
}

#[derive(Event)]
struct SoundEvent(SoundType);

impl Default for SoundEvent {
    fn default() -> Self {
        SoundEvent(SoundType::Silence)
    }
}

fn setup_once(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Camera
    commands.spawn(Camera2dBundle::default());

    // Sounds
    let mut sounds = Sounds::new();

    let grow_sound = asset_server.load("sounds/grow.mp3");
    sounds.add_sound(SoundType::Grow, grow_sound);

    let difficulty_up_sound = asset_server.load("sounds/difficulty_up.mp3");
    sounds.add_sound(SoundType::DifficultyUp, difficulty_up_sound);

    let failure_sound = asset_server.load("sounds/failure.mp3");
    sounds.add_sound(SoundType::Failure, failure_sound);

    commands.insert_resource(sounds);
}

fn setup(mut commands: Commands) {
    // Walls
    commands.spawn(WallBundle::new(WallLocation::Left, BLOCK_SIZE));
    commands.spawn(WallBundle::new(WallLocation::Top, BLOCK_SIZE));
    commands.spawn(WallBundle::new(WallLocation::Right, BLOCK_SIZE));
    commands.spawn(WallBundle::new(WallLocation::Bottom, BLOCK_SIZE));

    // Mouse
    commands.spawn(MouseBundle::new(BLOCK_SIZE));

    // Snake
    let delta = 1.0 / SNAKE_STARTING_LENGTH as f32;
    let blocks_offset = SNAKE_STARTING_DIRECTION.reverse().velocity();
    let mut color = SNAKE_COLOR;
    for i in 0..SNAKE_STARTING_LENGTH {
        color.set_r(delta * i as f32);

        commands.spawn(SnakeBundle::new(
            i as u32,
            BlockBundle::new(
                color,
                Position::new(
                    SNAKE_STARTING_POSITION.x + i as f32 * blocks_offset.x,
                    SNAKE_STARTING_POSITION.y + i as f32 * blocks_offset.y,
                ),
                BLOCK_SIZE,
            ),
            SNAKE_STARTING_DIRECTION,
        ));
    }

    // Scoreboard
    commands.spawn((
        TextBundle::from_sections([
            TextSection::new(
                "Score: ",
                TextStyle {
                    font_size: SCOREBOARD_FONT_SIZE,
                    color: SCOREBOARD_COLOR,
                    ..default()
                },
            ),
            TextSection::new(
                "0",
                TextStyle {
                    font_size: SCOREBOARD_FONT_SIZE,
                    color: SCOREBOARD_COLOR,
                    ..default()
                },
            ),
            TextSection::new(
                "\nDifficulty: ",
                TextStyle {
                    font_size: SCOREBOARD_FONT_SIZE,
                    color: SCOREBOARD_COLOR,
                    ..default()
                },
            ),
            TextSection::new(
                "0",
                TextStyle {
                    font_size: SCOREBOARD_FONT_SIZE,
                    color: SCOREBOARD_COLOR,
                    ..default()
                },
            ),
        ]).with_style(Style {
            position_type: PositionType::Absolute,
            top: SCOREBOARD_PADDING,
            left: SCOREBOARD_PADDING,
            ..default()
        }),
        ScoreboardComponent,
        GameComponents,
    ));
}

fn handle_state_input(
    keys: Res<Input<KeyCode>>,
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    match state.get() {
        GameState::Startup if keys.just_pressed(KeyCode::Space) => next_state.set(GameState::Running),
        GameState::Running if keys.just_pressed(KeyCode::Space) => next_state.set(GameState::Paused),
        GameState::Paused if keys.just_pressed(KeyCode::Space) => next_state.set(GameState::Running),
        GameState::GameOver if keys.just_pressed(KeyCode::R) => next_state.set(GameState::Running),
        _ => {}
    };
}

fn move_snake(
    keys: Res<Input<KeyCode>>,
    mut query: Query<(&mut Transform, &mut Position, &mut Direction), With<Snake>>,
    time: Res<Time>,
    mut timer: ResMut<MoveTimer>,
    mut direction_queue: Local<VecDeque<Direction>>,
) {
    timer.tick(time.delta());

    {
        // Handle keyboard controls
        let (_, _, mut head_dir) = query.iter_mut().next().unwrap();

        let directions: Vec<Direction> = keys.get_just_pressed().filter_map(|k| match k {
            KeyCode::Left | KeyCode::A => Some(Direction::Left),
            KeyCode::Right | KeyCode::D => Some(Direction::Right),
            KeyCode::Up | KeyCode::W => Some(Direction::Up),
            KeyCode::Down | KeyCode::S => Some(Direction::Down),
            _ => None,
        }).collect();

        for direction in &directions {
            if direction_queue.len() == MAX_INPUT_QUEUE_LENGTH {
                break;
            }

            direction_queue.push_back(*direction);
        }

        if timer.just_finished() {
            while !direction_queue.is_empty() {
                let d = direction_queue.pop_front().unwrap();

                if d.reverse() != *head_dir {
                    *head_dir = d;
                    break;
                }
            }
        }
    }

    // Move the snake
    if timer.just_finished() {
        let mut prev_dir = None;
        for (mut transform, mut pos, mut dir) in query.iter_mut() {
            pos.apply_vel(&dir.velocity());
            transform.translation = pos.translation();

            if let Some(d) = prev_dir {
                prev_dir = Some(dir.clone());
                *dir = d.clone();
            } else {
                prev_dir = Some(dir.clone());
            }
        }
    }
}

fn check_collisions(
    mut commands: Commands,
    mut scoreboard: ResMut<Scoreboard>,
    mut state: ResMut<NextState<GameState>>,
    mut sound_events: EventWriter<SoundEvent>,
    snake_query: Query<(&Snake, &Transform, &Position, &Direction), With<Snake>>,
    collider_query: Query<(Entity, &Transform, Option<&Snake>, Option<&Mouse>), With<Collider>>,
) {
    let snake: Vec<(&Snake, &Transform, &Position, &Direction)> = snake_query.iter().collect();

    let (head, head_transform, _, _) = snake.first().unwrap();

    for (entity, transform, maybe_snake, maybe_mouse) in collider_query.iter() {
        // Do not collide snake head with itself
        if let Some(snake) = maybe_snake {
            if snake.0 == head.0 {
                continue;
            }
        }

        let collision = collide(
            head_transform.translation,
            head_transform.scale.truncate(),
            transform.translation,
            transform.scale.truncate(),
        );

        if let Some(_) = collision {
            // If collided with mouse, spawn a new one
            if maybe_mouse.is_some() {
                scoreboard.score += SCORE_DELTA;

                commands.entity(entity).despawn();

                let mut mouse_bundle = MouseBundle::new(BLOCK_SIZE);
                // Check if we are trying to spawn a mouse inside the snake
                while snake.iter().find(|(_, _, position, _)| {
                    position.x == mouse_bundle.block_bundle.position.x
                        && position.y == mouse_bundle.block_bundle.position.y
                }).is_some() {
                    mouse_bundle = MouseBundle::new(BLOCK_SIZE);
                }

                commands.spawn(mouse_bundle);

                // Spawn a new snake block behind the current tail block
                let (tail, _, tail_position, &tail_direction) = snake.last().unwrap();
                let pos_offset = tail_direction.reverse().velocity();
                commands.spawn(SnakeBundle::new(
                    tail.0 + 1,
                    BlockBundle::new(
                        SNAKE_COLOR,
                        Position::new(
                            tail_position.x + pos_offset.x,
                            tail_position.y + pos_offset.y,
                        ),
                        BLOCK_SIZE,
                    ),
                    tail_direction,
                ));

                sound_events.send(SoundEvent(SoundType::Grow));

                return;
            }

            // If collided with wall or snake itself, stop the game
            state.set(GameState::GameOver);
        }
    }
}

fn update_scoreboard(scoreboard: Res<Scoreboard>, mut query: Query<&mut Text, With<ScoreboardComponent>>) {
    let mut text = query.single_mut();
    text.sections[1].value = scoreboard.score.to_string();
    text.sections[3].value = scoreboard.difficulty.to_string();
}

fn update_difficulty(
    mut scoreboard: ResMut<Scoreboard>,
    mut timer: ResMut<MoveTimer>,
    mut sound_events: EventWriter<SoundEvent>,
) {
    let difficulty = (scoreboard.score as f32 / SCORE_DIFFICULTY_THRESHOLD).floor() as usize;

    if difficulty != scoreboard.difficulty {
        scoreboard.difficulty = difficulty;

        let new_duration = timer.duration().as_secs_f32() * (1.0 - TIMER_SCALING_PERCENTAGE / 100.0);

        timer.set_duration(Duration::from_secs_f32(new_duration));

        sound_events.send(SoundEvent(SoundType::DifficultyUp));
    }
}

fn play_sounds(
    mut commands: Commands,
    mut sound_events: EventReader<SoundEvent>,
    sounds: Res<Sounds>,
) {
    if !sound_events.is_empty() {
        for sound_event in sound_events.read() {
            if let Some(sound) = sounds.get_sound(&sound_event.0) {
                commands.spawn(AudioBundle {
                    source: sound,
                    settings: PlaybackSettings::DESPAWN,
                });
            }
        }
    }
}

fn game_over(mut sound_events: EventWriter<SoundEvent>) {
    sound_events.send(SoundEvent(SoundType::Failure))
}

fn despawn<T: Component>(mut commands: Commands, query: Query<Entity, With<T>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}

fn reset(mut scoreboard: ResMut<Scoreboard>, mut timer: ResMut<MoveTimer>) {
    scoreboard.score = 0;
    scoreboard.difficulty = 0;
    timer.set_duration(Duration::from_secs_f32(TIMER_STARTING_DURATION));
}

#[derive(Component, Default)]
struct StartupMessage;

impl Message for StartupMessage {
    fn get_message() -> String {
        String::from(r#"USE WASD OR ARROW KEYS TO CONTROL THE SNAKE
PRESS SPACE TO PAUSE OR UNPAUSE THE GAME
PRESS ESC TO EXIT
PRESS SPACE TO CONTINUE"#)
    }
}

#[derive(Component, Default)]
struct PausedMessage;

impl Message for PausedMessage {
    fn get_message() -> String {
        String::from("PAUSED")
    }
}

#[derive(Component, Default)]
struct GameOverMessage;

impl Message for GameOverMessage {
    fn get_message() -> String {
        String::from("GAME OVER\nPRESS R TO RESTART OR ESC TO EXIT")
    }
}

trait Message {
    fn get_message() -> String;
}

fn spawn_message<T: Component + Message + Default>(mut commands: Commands) {
    commands
        .spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: MESSAGE_BOX_BACKGROUND_COLOR,
                    custom_size: Some(MESSAGE_BOX_SIZE),
                    ..default()
                },
                transform: Transform::from_translation(Vec3::Z),
                ..default()
            },
            T::default(),
        ))
        .with_children(|builder| {
            builder.spawn((
                Text2dBundle {
                    text: Text {
                        sections: vec![TextSection::new(
                            T::get_message(),
                            TextStyle {
                                font_size: MESSAGE_BOX_FONT_SIZE,
                                color: MESSAGE_BOX_TEXT_COLOR,
                                ..default()
                            },
                        )],
                        alignment: TextAlignment::Center,
                        ..default()
                    },
                    text_2d_bounds: Text2dBounds {
                        size: MESSAGE_BOX_SIZE,
                    },
                    transform: Transform::from_translation(Vec3::Z * Vec3::splat(2.0)),
                    ..default()
                },
            ));
        });
}
