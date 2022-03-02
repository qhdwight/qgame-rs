use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
};
use std::option::Option;

#[derive(Component)]
pub struct Item {
    pub name: String,
    pub amount: u16,
}

#[derive(Component)]
pub struct Gun {
    pub ammo: u16,
    pub ammo_in_reserve: u16,
}

#[derive(Component)]
pub struct Inventory {
    pub equipped_idx: Option<usize>,
    pub previous_equipped_idx: Option<usize>,
}

pub fn manage_inventory(
    time: Res<Time>,
    mut query: Query<(&mut Item, With<Inventory>)>,
) {
    for (mut inventory) in query.iter_mut() {

    }
}