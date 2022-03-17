use std::{
    f32::consts::TAU,
    option::Option,
    time::Duration,
};
use std::ops::MulAssign;

use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    core_pipeline::node::MAIN_PASS_DEPENDENCIES,
    pbr::{DrawMesh, MeshPipeline, MeshPipelineKey, SetMeshBindGroup, SetMeshViewBindGroup},
    prelude::*,
    reflect::TypeUuid,
    render::{RenderApp, RenderStage},
    render::render_graph::RenderGraph,
    render::render_phase::{EntityRenderCommand, SetItemPipeline},
    render::render_resource::{RenderPipelineDescriptor, SpecializedPipeline},
    utils::{BoxedFuture, HashMap},
};
use bevy_rapier3d::prelude::*;
use serde::{Deserialize, Serialize};
use smartstring::alias::String;

use crate::{DefaultMaterials, PlayerInput};

const EQUIPPING_STATE: &str = "equipping";
const EQUIPPED_STATE: &str = "equipped";
const UNEQUIPPING_STATE: &str = "unequipping";
const UNEQUIPPED_STATE: &str = "unequipped";
const IDLE_STATE: &str = "idle";
const RELOAD_STATE: &str = "reload";
const FIRE_STATE: &str = "fire";

pub type ItemName = String;
type ItemStateName = String;
type EquipStateName = String;

#[derive(Serialize, Deserialize)]
pub struct ItemStateProps {
    pub duration: Duration,
    pub is_persistent: bool,
}

#[derive(Serialize, Deserialize, TypeUuid)]
#[uuid = "2cc54620-95c6-4522-b40e-0a4991ebae5f"]
pub struct ItemProps {
    pub name: ItemName,
    pub move_factor: f32,
    pub states: HashMap<ItemStateName, ItemStateProps>,
    pub equip_states: HashMap<EquipStateName, ItemStateProps>,
}

#[derive(Serialize, Deserialize, TypeUuid)]
#[uuid = "46e9c7af-27c2-4560-86e7-df48f9e84729"]
pub struct WeaponProps {
    pub damage: u16,
    pub headshot_factor: f32,
    pub item_props: ItemProps,
}

#[derive(Serialize, Deserialize, TypeUuid)]
#[uuid = "df56751c-7560-420d-b480-eb8fb6f9b9bf"]
pub struct GunProps {
    pub mag_size: u16,
    pub starting_ammo_in_reserve: u16,
    pub weapon_props: WeaponProps,
}

#[derive(Component, Debug)]
pub struct Item {
    pub name: ItemName,
    pub amount: u16,
    pub state_name: ItemStateName,
    pub state_dur: Duration,
    pub inv_ent: Entity,
    pub inv_slot: u8,
}

#[derive(Component)]
pub struct ItemPickup {
    pub item_name: ItemName,
}

#[derive(Component, Default)]
pub struct ItemPickupVisual;

#[derive(Component)]
pub struct Gun {
    pub ammo: u16,
    pub ammo_in_reserve: u16,
}

#[derive(Debug)]
pub struct Items(pub [Option<Entity>; 10]);

#[derive(Component)]
pub struct ItemVisual;

#[derive(Component, Debug)]
pub struct Inventory {
    pub equipped_slot: Option<u8>,
    pub prev_equipped_slot: Option<u8>,
    pub equip_state_name: EquipStateName,
    pub equip_state_dur: Duration,
    pub item_ents: Items,
}

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {}
}

#[derive(Default)]
pub struct ConfigAssetLoader;

impl AssetLoader for ConfigAssetLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            let asset: GunProps = toml::from_slice(bytes)?;
            load_context.set_default_asset(LoadedAsset::new(asset));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["config.toml"]
    }
}

// ███╗   ███╗ ██████╗ ██████╗ ██╗███████╗██╗   ██╗
// ████╗ ████║██╔═══██╗██╔══██╗██║██╔════╝╚██╗ ██╔╝
// ██╔████╔██║██║   ██║██║  ██║██║█████╗   ╚████╔╝
// ██║╚██╔╝██║██║   ██║██║  ██║██║██╔══╝    ╚██╔╝
// ██║ ╚═╝ ██║╚██████╔╝██████╔╝██║██║        ██║
// ╚═╝     ╚═╝ ╚═════╝ ╚═════╝ ╚═╝╚═╝        ╚═╝

pub fn modify_equip_state_sys(
    time: Res<Time>,
    asset_server: Res<AssetServer>,
    mut inv_query: Query<(&PlayerInput, &mut Inventory)>,
    mut item_query: Query<&mut Item>,
) {
    for (input, mut inv) in inv_query.iter_mut() {
        let input: &PlayerInput = input;
        let mut inv: Mut<'_, Inventory> = inv;

        let has_valid_wanted = input.wanted_item_slot.is_some()
            && inv.item_ents.0[input.wanted_item_slot.unwrap() as usize].is_some();

        // Handle unequipping current item
        let is_alr_unequipping = inv.equip_state_name == UNEQUIPPING_STATE;
        if has_valid_wanted && input.wanted_item_slot != inv.equipped_slot && !is_alr_unequipping {
            inv.equip_state_name = String::from(UNEQUIPPING_STATE);
            inv.equip_state_dur = Duration::ZERO;
        }
        if inv.equipped_slot.is_none() { return; }
        if inv.equipped_slot.is_none() { return; }

        // Handle finishing equip status
        inv.equip_state_dur = inv.equip_state_dur.saturating_add(time.delta());
        while inv.equip_state_dur > Duration::from_millis(2000) {
            match inv.equip_state_name.as_str() {
                EQUIPPING_STATE => {
                    inv.equip_state_name = String::from(EQUIPPED_STATE);
                }
                UNEQUIPPING_STATE => {
                    inv.equip_state_name = String::from(UNEQUIPPED_STATE);
                }
                _ => {}
            }
            inv.equip_state_dur = inv.equip_state_dur.saturating_sub(Duration::from_millis(2000));
        }

        if inv.equip_state_name != UNEQUIPPED_STATE { return; }

        // We have unequipped the last slot, so we need to starting equipping the new slot
        if has_valid_wanted {
            inv.prev_equipped_slot = inv.equipped_slot;
            inv.equipped_slot = input.wanted_item_slot;
        } else {
            inv.equipped_slot = inv.find_replacement(&mut item_query);
        }
        inv.equip_state_name = String::from(EQUIPPING_STATE);
    }
}

pub fn modify_item_sys(
    time: Res<Time>,
    mut item_query: Query<&mut Item>,
    inv_query: Query<&Inventory>,
) {
    for mut item in item_query.iter_mut() {
        let is_equipped = inv_query.get(item.inv_ent).unwrap().equipped_slot == Some(item.inv_slot);
        if is_equipped {
            item.state_dur = item.state_dur.saturating_add(time.delta());
            while item.state_dur > Duration::from_millis(2000) {
                match item.state_name.as_str() {
                    IDLE_STATE | RELOAD_STATE | FIRE_STATE => {
                        item.state_name = String::from(IDLE_STATE);
                    }
                    _ => unimplemented!()
                }
                item.state_dur = item.state_dur.saturating_sub(Duration::from_millis(2000));
            }
        }
    }
}

pub fn item_pickup_sys(
    mut commands: Commands,
    // query_pipeline: Res<QueryPipeline>,
    // collider_query: QueryPipelineColliderComponentsQuery,
    // mut inv_query: Query<(&mut Inventory, &ColliderShapeComponent)>,
    mut intersection_events: EventReader<IntersectionEvent>,
    mut inv_query: Query<&mut Inventory>,
    mut item_query: Query<&mut Item>,
    mut pickup_query: Query<&mut ItemPickup>,
) {
    // TODO:design use shape cast instead of reading events?
    // let collider_set = QueryPipelineColliderComponentsSet(&collider_query);
    //
    // for (mut inv, player_collider) in inv_query.iter_mut() {
    //     let mut inv: Mut<'_, Inventory> = inv;
    //     let player_collider: &ColliderShapeComponent = player_collider;
    //
    //     query_pipeline.intersections_with_shape(&collider_set, )
    // }
    for intersection_event in intersection_events.iter() {
        let intersection: &IntersectionEvent = intersection_event;
        let ent1 = intersection.collider1.entity();
        let ent2 = intersection.collider2.entity();
        let mut pickup_ent: Option<Entity> = None;
        let mut player_ent: Option<Entity> = None;
        if pickup_query.get(ent1).is_ok() && inv_query.get(ent2).is_ok() {
            pickup_ent = Some(ent1);
            player_ent = Some(ent2);
        } else if pickup_query.get(ent2).is_ok() && inv_query.get(ent1).is_ok() {
            pickup_ent = Some(ent2);
            player_ent = Some(ent1);
        }
        if let Some(pickup_ent) = pickup_ent {
            if let Some(player_ent) = player_ent {
                let pickup = pickup_query.get_mut(pickup_ent).unwrap();
                let mut inv = inv_query.get_mut(player_ent).unwrap();
                inv.insert_item(player_ent, &mut commands, &mut item_query, &pickup.item_name);
                commands.entity(pickup_ent).despawn_recursive();
            }
        }
    }
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            equipped_slot: None,
            prev_equipped_slot: None,
            equip_state_name: String::from(UNEQUIPPED_STATE),
            equip_state_dur: Duration::ZERO,
            item_ents: Items([None; 10]),
        }
    }
}

impl Inventory {
    fn start_item_state(&self, mut item: Mut<Item>, state: ItemStateName, dur: Duration) {
        item.state_name = state;
        item.state_dur = dur;
        match item.state_name {
            _ => unimplemented!()
        }
    }

    fn find_replacement(&self, item_query: &mut Query<&mut Item>) -> Option<u8> {
        if self.prev_equipped_slot.is_none() {
            self.find_slot(item_query, |item| item.is_some())
        } else {
            self.prev_equipped_slot
        }
    }

    fn find_slot(
        &self, item_query: &mut Query<&mut Item>, predicate: impl Fn(Option<&Item>) -> bool,
    ) -> Option<u8> {
        for (slot, &item_ent) in self.item_ents.0.iter().enumerate() {
            let slot = slot as u8;
            let item = match item_ent {
                Some(item_ent) => item_query.get(item_ent),
                None => Err(bevy::ecs::query::QueryEntityError::NoSuchEntity),
            }.ok();
            if predicate(item) {
                return Some(slot);
            }
        }
        None
    }

    pub fn insert_item(
        &mut self,
        inv_ent: Entity,
        commands: &mut Commands,
        item_query: &mut Query<&mut Item>,
        item_name: &ItemName,
    ) {
        let open_slot = self.find_slot(item_query, |item| item.is_none());
        if let Some(open_slot) = open_slot {
            self.set_item(inv_ent, commands, item_name, open_slot);
        }
    }

    pub fn set_item(
        &mut self,
        inv_ent: Entity,
        commands: &mut Commands,
        item_name: &ItemName, slot: u8,
    ) -> &mut Self {
        let existing_item_ent = self.item_ents.0[slot as usize];
        if let Some(existing_item_ent) = existing_item_ent {
            commands.entity(existing_item_ent).despawn()
        }
        let item_ent = commands.spawn()
            .insert(Item {
                name: item_name.clone(),
                amount: 1,
                state_name: String::from(IDLE_STATE),
                state_dur: Duration::ZERO,
                inv_ent,
                inv_slot: slot,
            }).id();
        if self.equipped_slot.is_none() {
            self.equipped_slot = Some(slot);
            self.equip_state_dur = Duration::ZERO;
            self.equip_state_name = String::from(EQUIPPING_STATE);
        }
        self.item_ents.0[slot as usize] = Some(item_ent);
        self
    }
}

// ██████╗ ███████╗███╗   ██╗██████╗ ███████╗██████╗
// ██╔══██╗██╔════╝████╗  ██║██╔══██╗██╔════╝██╔══██╗
// ██████╔╝█████╗  ██╔██╗ ██║██║  ██║█████╗  ██████╔╝
// ██╔══██╗██╔══╝  ██║╚██╗██║██║  ██║██╔══╝  ██╔══██╗
// ██║  ██║███████╗██║ ╚████║██████╔╝███████╗██║  ██║
// ╚═╝  ╚═╝╚══════╝╚═╝  ╚═══╝╚═════╝ ╚══════╝╚═╝  ╚═╝

pub fn render_inventory_sys(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    materials: Res<DefaultMaterials>,
    item_query: Query<&mut Item>,
    player_query: Query<&Inventory>,
    camera_query: Query<&Transform, With<PerspectiveProjection>>,
) {
    for inv in player_query.iter() {
        for item in inv.item_ents.0.iter() {
            if let Some(item_ent) = item {
                if let Ok(item) = item_query.get(*item_ent) {
                    let is_equipped = inv.equipped_slot == Some(item.inv_slot);
                    let mut transform = Transform::default();
                    let mesh_handle = asset_server.load(format!("models/{}.gltf#Mesh0/Primitive0", item.name).as_str());
                    if is_equipped {
                        transform = camera_query.single().clone();
                        transform.translation += transform.rotation * Vec3::new(0.4, -0.3, -1.0);
                        transform.rotation.mul_assign(Quat::from_rotation_y(TAU / 2.0));
                    }
                    commands.entity(*item_ent).insert_bundle(PbrBundle {
                        mesh: mesh_handle.clone(),
                        material: materials.gun_material.clone(),
                        transform,
                        visibility: Visibility { is_visible: is_equipped },
                        ..Default::default()
                    });
                }
            }
        }
    }
}

pub fn item_pickup_animate_sys(
    time: Res<Time>,
    mut pickup_query: Query<&mut Transform, With<ItemPickupVisual>>,
) {
    for mut transform in pickup_query.iter_mut() {
        let dr = TAU * time.delta_seconds() * 0.125;
        transform.rotate(Quat::from_axis_angle(Vec3::Y, dr));
        let height = f32::sin(time.time_since_startup().as_secs_f32()) * 0.125;
        transform.translation = Vec3::new(0.0, height, 0.0);
    }
}
