extern crate core;

use std::{
    f32::consts::TAU,
    fmt::Write,
};

use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
    prelude::shape::Cube,
    render::{
        mesh::{Indices, VertexAttributeValues},
        render_resource::*,
    },
};
use bevy::render::view::NoFrustumCulling;
use bevy_rapier3d::prelude::*;

use qgame::*;

mod qgame;

#[derive(Component)]
struct TopRightText;

#[derive(Component)]
struct PlayerHudText;

#[derive(Resource)]
pub struct DefaultMaterials {
    pub gun_material: Handle<StandardMaterial>,
}

#[derive(Clone, Hash, Debug, PartialEq, Eq, SystemSet)]
pub enum PlayerSet {
    Logic,
    Render,
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 0.25,
        })
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            watch_for_changes: true,
            ..default()
        }))
        .insert_resource(RapierConfiguration {
            ..default()
        })
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        // .add_plugin(RapierDebugRenderPlugin::default())
        .add_plugin(VoxelsPlugin)
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(InventoryPlugin)
        .add_asset::<Config>()
        .init_asset_loader::<ConfigAssetLoader>()
        .add_startup_systems((setup_sys, spawn_ui_sys, spawn_voxel_sys, spawn_player_sys))
        .add_systems((player_input_system.in_base_set(CoreSet::PreUpdate), cursor_grab_sys, update_fps_text_sys))
        .add_systems((player_look_sys, player_move_sys, modify_equip_state_sys, modify_item_sys, item_pickup_sys).chain().in_set(PlayerSet::Logic))
        .add_systems((item_pickup_animate_sys, render_player_camera_sys, render_inventory_sys, update_hud_system).chain().in_set(PlayerSet::Render))
        .run();
}

fn setup_sys(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // println!("{}", toml::to_string(&Config::default()).unwrap());

    let config: Handle<Config> = asset_server.load("default.config.toml");
    commands.insert_resource(ConfigState { handle: config });

    // commands.spawn_bundle(PointLightBundle {
    //     point_light: PointLight {
    //         intensity: 2000.0,
    //         shadows_enabled: true,
    //         ..default()
    //     },
    //     transform: Transform::from_xyz(38.0, -34.0, 40.0),
    //     ..default()
    // });

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 2000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(-38.0, 40.0, 34.0),
        ..default()
    });

    {
        let mesh = meshes.add(Mesh::from(Cube { size: 1.0 }));
        let material = materials.add(StandardMaterial {
            base_color: Color::PINK,
            ..default()
        });
        commands.spawn((
            PbrBundle {
                mesh: mesh.clone(),
                material: material.clone(),
                // transform: Transform::from_xyz(-18.0, 32.0, -18.0),
                transform: Transform::from_xyz(0.0, 0.0, 0.0),
                ..default()
            },
            Collider::cuboid(0.5, 0.5, 0.5),
        ));
    }

    let gun_material = materials.add(StandardMaterial {
        base_color: Color::DARK_GRAY,
        metallic: 0.05,
        perceptual_roughness: 0.1,
        ..default()
    });

    // let rifle_handle = asset_server.load("models/rifle.gltf#Mesh0/Primitive0");
    // commands.spawn()
    //     .insert(GlobalTransform::default())
    //     .with_children(|parent| {
    //         parent.spawn_bundle(PbrBundle {
    //             mesh: rifle_handle.clone(),
    //             material: gun_material.clone(),
    //             ..default()
    //         })
    //             .insert(ItemPickupVisual::default());
    //     })
    //     .insert(Collider::ball(0.5))
    //     .insert(Sensor(true))
    //     .insert(Transform::from_xyz(0.0, 20.0, 8.0))
    //     .insert(ItemPickup { item_name: ItemName::from("rifle") });

    commands.insert_resource(DefaultMaterials { gun_material });
}

fn spawn_ui_sys(asset_server: Res<AssetServer>, mut commands: Commands) {
    let font = asset_server.load("fonts/FiraMono-Medium.ttf");

    commands
        .spawn(TextBundle {
            style: Style {
                align_self: AlignSelf::FlexEnd,
                position_type: PositionType::Absolute,
                position: UiRect {
                    top: Val::Px(5.0),
                    right: Val::Px(5.0),
                    ..default()
                },
                ..default()
            },
            text: Text {
                sections: vec![
                    TextSection {
                        value: "".to_string(),
                        style: TextStyle { font: font.clone(), font_size: 16.0, color: Color::WHITE },
                    },
                ],
                ..default()
            },
            ..default()
        })
        .insert(TopRightText);

    commands.spawn((
        TextBundle {
            style: Style {
                align_self: AlignSelf::FlexEnd,
                position_type: PositionType::Absolute,
                position: UiRect {
                    bottom: Val::Px(5.0),
                    left: Val::Px(5.0),
                    ..default()
                },
                ..default()
            },
            text: Text {
                sections: vec![
                    TextSection {
                        value: "".to_string(),
                        style: TextStyle { font: font.clone(), font_size: 12.0, color: Color::ANTIQUE_WHITE },
                    },
                ],
                ..default()
            },
            ..default()
        },
        PlayerHudText
    ));
}

fn spawn_voxel_sys(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.set_indices(Some(Indices::U32(Vec::with_capacity(4096))));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, VertexAttributeValues::Float32x3(Vec::with_capacity(4096)));
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, VertexAttributeValues::Float32x3(Vec::with_capacity(4096)));
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, VertexAttributeValues::Float32x2(Vec::with_capacity(4096)));
    let mesh_handle = meshes.add(mesh);
    let ground_mat_handle = materials.add(StandardMaterial {
        base_color: Color::DARK_GREEN,
        ..default()
    });
    commands.spawn(Map::default());
    commands.spawn((
        Chunk::new(IVec3::ZERO),
        NoFrustumCulling,
        PbrBundle {
            mesh: mesh_handle.clone(),
            material: ground_mat_handle.clone(),
            ..default()
        },
    ));
}

fn spawn_player_sys(mut commands: Commands) {
    commands.spawn((
        Collider::capsule(Vec3::Y * 0.5, Vec3::Y * 1.5, 0.5),
        Velocity::zero(),
        RigidBody::Dynamic,
        Sleeping::disabled(),
        LockedAxes::ROTATION_LOCKED,
        ReadMassProperties(MassProperties {
            mass: 1.0,
            ..default()
        }),
        GravityScale(0.0),
        Ccd { enabled: true },
        TransformBundle::from(Transform::from_xyz(4.0, 18.0, 4.0)),
        LogicalPlayer(0),
        PlayerInput {
            pitch: -TAU / 12.0,
            yaw: TAU * 5.0 / 8.0,
            ..default()
        },
        PlayerController {
            ..default()
        },
        Inventory::default(),
    ));

    commands.spawn((Camera3dBundle::default(), RenderPlayer(0)));
}

#[derive(Resource)]
pub struct Buffers {
    // Place edge table and triangle table in uniform buffer
    // They are too large to have inline in the shader
    triangle_table: Buffer,
    block_face_table: Buffer,
    points: BufVec<Vec2>,
    heights: BufVec<f32>,
    voxels: Buffer,
    voxels_staging: Buffer,
    vertices: BufVec<Vec4>,
    normals: BufVec<Vec4>,
    uvs: BufVec<Vec2>,
    indices: BufVec<u32>,
    atomics: BufVec<u32>,
    atomics_staging: Buffer,
}

struct BindingGroups {
    simplex: BindGroup,
    voxels: BindGroup,
}

fn update_fps_text_sys(
    diagnostics: Res<Diagnostics>,
    mut query: Query<&mut Text, With<TopRightText>>,
) {
    for mut text in query.iter_mut() {
        let mut fps = 0.0;
        if let Some(fps_diagnostic) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(fps_avg) = fps_diagnostic.average() {
                fps = fps_avg;
            }
        }

        let mut frame_time = 0.0f64;
        if let Some(frame_time_diagnostic) = diagnostics.get(FrameTimeDiagnosticsPlugin::FRAME_TIME) {
            if let Some(frame_time_avg) = frame_time_diagnostic.average() {
                frame_time = frame_time_avg;
            }
        }

        let text = &mut text.sections[0].value;
        text.clear();
        write!(text, "{:.1} fps, {:.3} ms/frame", fps, frame_time).unwrap();
    }
}

fn update_hud_system(
    mut text_query: Query<&mut Text, With<PlayerHudText>>,
    player_query: Query<&Transform, With<PerspectiveProjection>>,
    mut item_query: Query<&mut Item>,
    inv_query: Query<(&Inventory, &PlayerInput)>,
) {
    for mut text in text_query.iter_mut() {
        let text = &mut text.sections[0].value;
        text.clear();
        for transform in player_query.iter() {
            let p = transform.translation;
            write!(text, "Position {{ {:.2}, {:.2}, {:.2} }}", p.x, p.y, p.z).unwrap();
        }
        for (inv, input) in inv_query.iter() {
            write!(text, "\n{:?}", input).unwrap();
            write!(text, "\n{:?}", inv).unwrap();
            for i in 0..inv.item_ents.0.len() {
                if let Some(item_ent) = inv.item_ents.0[i] {
                    if let Ok(item) = item_query.get_mut(item_ent) {
                        write!(text, "\n{:?}", *item).unwrap();
                    }
                }
            }
        }
    }
}
