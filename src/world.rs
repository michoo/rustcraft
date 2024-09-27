use crate::constants::{CHUNK_RENDER_DISTANCE_RADIUS, CHUNK_SIZE};
use crate::materials::{MaterialResource, MeshResource};
use crate::utils::{global_block_to_chunk_pos, to_global_pos, to_local_pos, SIX_OFFSETS};
use crate::BlockRaycastSet;
use bevy::prelude::Resource;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy_mod_raycast::prelude::*;
use noise::{NoiseFn, Perlin};
use rand::Rng;
use std::collections::HashMap;

#[derive(Component)]
pub struct BlockMarker;

#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
pub enum Block {
    Grass,
    Dirt,
    Stone,
    Bedrock,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone, Copy)]
pub enum GlobalMaterial {
    Sun,
    Moon,
}

#[derive(Component, Clone)]
pub struct BlockWrapper {
    pub kind: Block,
}

#[derive(Resource)]
pub struct WorldSeed(pub u32);

#[derive(Clone, Default)]
pub struct Chunk {
    map: HashMap<IVec3, BlockWrapper>,
    entity: Option<Entity>,
}

#[derive(Resource, Default, Clone)]
pub struct WorldMap {
    pub map: HashMap<IVec3, Chunk>,
    pub total_blocks_count: u64,
    pub total_chunks_count: u64,
}

pub fn block_to_chunk_coord(x: i32) -> i32 {
    if x >= 0 {
        x / CHUNK_SIZE
    } else {
        (x - (CHUNK_SIZE - 1)) / CHUNK_SIZE
    }
}

pub fn block_vec3_to_chunk_v3_coord(v: Vec3) -> Vec3 {
    Vec3::new(
        block_to_chunk_coord(v.x as i32) as f32,
        block_to_chunk_coord(v.y as i32) as f32,
        block_to_chunk_coord(v.z as i32) as f32,
    )
}

impl WorldMap {
    pub fn get_block_by_coordinates(&self, position: &IVec3) -> Option<&BlockWrapper> {
        let x = position.x;
        let y = position.y;
        let z = position.z;
        let cx = block_to_chunk_coord(x);
        let cy = block_to_chunk_coord(y);
        let cz = block_to_chunk_coord(z);
        let chunk = self.map.get(&IVec3::new(cx, cy, cz));
        match chunk {
            Some(chunk) => {
                let sub_x = ((x % CHUNK_SIZE) + CHUNK_SIZE) % CHUNK_SIZE;
                let sub_y = ((y % CHUNK_SIZE) + CHUNK_SIZE) % CHUNK_SIZE;
                let sub_z = ((z % CHUNK_SIZE) + CHUNK_SIZE) % CHUNK_SIZE;
                chunk.map.get(&IVec3::new(sub_x, sub_y, sub_z))
            }
            None => None,
        }
    }

    pub fn remove_block_by_coordinates(&mut self, global_block_pos: &IVec3) -> Option<Block> {
        let block = self.get_block_by_coordinates(global_block_pos)?;
        let kind = block.kind;

        let chunk_pos = global_block_to_chunk_pos(global_block_pos);

        let chunk_map = self
            .map
            .get_mut(&IVec3::new(chunk_pos.x, chunk_pos.y, chunk_pos.z))?;

        let local_block_pos = to_local_pos(global_block_pos);

        chunk_map.map.remove(&local_block_pos);

        Some(kind)
    }

    pub fn set_block(&mut self, position: &IVec3, block: Block) {
        let x = position.x;
        let y = position.y;
        let z = position.z;
        let cx = block_to_chunk_coord(x);
        let cy = block_to_chunk_coord(y);
        let cz = block_to_chunk_coord(z);
        let chunk = self.map.entry(IVec3::new(cx, cy, cz)).or_default();
        let sub_x = ((x % CHUNK_SIZE) + CHUNK_SIZE) % CHUNK_SIZE;
        let sub_y = ((y % CHUNK_SIZE) + CHUNK_SIZE) % CHUNK_SIZE;
        let sub_z = ((z % CHUNK_SIZE) + CHUNK_SIZE) % CHUNK_SIZE;

        if x == 0 && z == 0 {
            println!("inserting y={}", y)
        }
        chunk.map.insert(
            IVec3::new(sub_x, sub_y, sub_z),
            BlockWrapper { kind: block },
        );
    }
}

fn generate_chunk(
    chunk_pos: IVec3,
    seed: u32,
    world_map: &mut WorldMap,
    ev_render: &mut EventWriter<WorldRenderRequestUpdateEvent>,
) {
    println!("gen chunk {}", chunk_pos);
    let perlin = Perlin::new(seed);

    let scale = 0.1;
    let max_perlin_height_variation = 5.0;
    let base_height = 10; // should be 64

    const WORLD_MIN_Y: i32 = 0;

    let cx = chunk_pos.x;
    let cz = chunk_pos.z;

    // Boucle pour générer les blocs avec variation de hauteur
    for i in 0..CHUNK_SIZE {
        for j in 0..CHUNK_SIZE {
            let x = CHUNK_SIZE * cx + i;
            let z = CHUNK_SIZE * cz + j;

            // Générer une hauteur en utilisant le bruit de Perlin
            let perlin_height =
                perlin.get([x as f64 * scale, z as f64 * scale]) * max_perlin_height_variation;

            // Ajouter un offset de 64 blocs pour centrer la hauteur autour de y = 64
            let terrain_height = base_height + perlin_height.round() as i32;

            // Générer des blocs à partir de la couche 0 (bedrock) jusqu'à la hauteur générée
            for y in WORLD_MIN_Y..=terrain_height {
                let block = if y == 0 {
                    Block::Bedrock // Placer la bedrock à la couche 0
                } else if y < terrain_height - 2 {
                    Block::Stone // Placer de la pierre en dessous des 3 dernières couches
                } else if y < terrain_height {
                    Block::Dirt // Placer de la terre dans les 3 couches sous la surface
                } else {
                    Block::Grass // Placer de l'herbe à la surface
                };

                world_map.set_block(&IVec3::new(x, y, z), block);

                // Incrémenter le compteur de blocs
                world_map.total_blocks_count += 1;
            }
        }
    }

    world_map.total_chunks_count += 1;
    ev_render.send(WorldRenderRequestUpdateEvent::ChunkToReload(chunk_pos));
}

pub fn setup_world(
    mut commands: Commands,
    mut world_map: ResMut<WorldMap>,
    mut ev_render: EventWriter<WorldRenderRequestUpdateEvent>,
) {
    let seed = rand::thread_rng().gen::<u32>();
    println!("Generated random seed: {}", seed);

    commands.insert_resource(WorldSeed(seed));

    for x in -1..=1 {
        for z in -1..=1 {
            generate_chunk(IVec3::new(x, 0, z), seed, &mut world_map, &mut ev_render);
        }
    }
}

pub fn load_chunk_around_player(
    player_position: Vec3,
    world_map: &mut WorldMap,
    seed: u32,
    ev_render: &mut EventWriter<WorldRenderRequestUpdateEvent>,
) {
    let player_chunk = IVec3::new(
        block_to_chunk_coord(player_position.x as i32),
        0,
        block_to_chunk_coord(player_position.z as i32),
    );

    let r = CHUNK_RENDER_DISTANCE_RADIUS;

    for x in -r..=r {
        for z in -r..=r {
            let chunk_pos = IVec3::new(player_chunk.x + x, 0, player_chunk.z + z);
            {
                let chunk = world_map.map.get(&chunk_pos);
                if chunk.is_some() {
                    continue;
                }
                // Doing these scoping shenanigans to release the Mutex at the end of the scope
                // because generate_chunk requires a Mutex lock as well
            }
            generate_chunk(chunk_pos, seed, world_map, ev_render);
        }
    }
}

fn should_block_be_rendered(
    world_map: &WorldMap,
    chunk_pos: &IVec3,
    local_block_pos: &IVec3,
) -> bool {
    let global_block_pos = to_global_pos(chunk_pos, local_block_pos);

    for offset in &SIX_OFFSETS {
        let neighbor_pos = global_block_pos + *offset;

        // Check if the block exists at the neighboring position
        if world_map.get_block_by_coordinates(&neighbor_pos).is_none() {
            return true;
        }
    }

    false
}

#[derive(Event)]
pub enum WorldRenderRequestUpdateEvent {
    ChunkToReload(IVec3),
    BlockToReload(IVec3),
}

fn generate_chunk_mesh(world_map: &WorldMap, chunk_pos: &IVec3) -> Mesh {
    let mut vertices: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();

    let mut indices_offset = 0;

    for x in 0..CHUNK_SIZE {
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let local_block_pos = IVec3::new(x, y, z);
                let x = x as f32;
                let y = y as f32;
                let z = z as f32;

                let block =
                    world_map.get_block_by_coordinates(&to_global_pos(chunk_pos, &local_block_pos));

                if block.is_none() {
                    continue;
                }

                println!("{} {} {}", x, y, z);

                // Define vertices for a cube in the range [0,1]
                let local_vertices: Vec<[f32; 3]> = vec![
                    // Front face
                    [0.0, 0.0, 1.0], // 0
                    [1.0, 0.0, 1.0], // 1
                    [1.0, 1.0, 1.0], // 2
                    [0.0, 1.0, 1.0], // 3
                    // Back face
                    [0.0, 0.0, 0.0], // 4
                    [1.0, 0.0, 0.0], // 5
                    [1.0, 1.0, 0.0], // 6
                    [0.0, 1.0, 0.0], // 7
                ]
                .iter()
                .map(|v| [v[0] + x, v[1] + y, v[2] + z])
                .collect();

                let local_indices: Vec<u32> = vec![
                    0, 1, 2, 0, 2, 3, // Front face
                    1, 5, 6, 1, 6, 2, // Right face
                    5, 4, 7, 5, 7, 6, // Back face
                    4, 0, 3, 4, 3, 7, // Left face
                    3, 2, 6, 3, 6, 7, // Top face
                    4, 5, 1, 4, 1, 0, // Bottom face
                ]
                .iter()
                .map(|x| x + indices_offset)
                .collect();

                let local_normals = vec![
                    // Normals for each vertex
                    [0.0, 0.0, 1.0],
                    [0.0, 0.0, 1.0],
                    [0.0, 0.0, 1.0],
                    [0.0, 0.0, 1.0], // Front face normals
                    [1.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0], // Right face normals
                ];

                indices_offset += local_vertices.len() as u32;

                // UV coordinates
                let local_uvs = vec![
                    // Front face (top-left texture in atlas)
                    [0.0, 0.0], // Bottom-left
                    [0.5, 0.0], // Bottom-right
                    [0.5, 0.5], // Top-right
                    [0.0, 0.5], // Top-left
                    // Right face (top-right texture in atlas)
                    [0.5, 0.0], // Bottom-left (duplicated vertex with new UV)
                    [1.0, 0.0], // Bottom-right
                    [1.0, 0.5], // Top-right
                    [0.5, 0.5], // Top-left (duplicated vertex with new UV)
                ];

                vertices.extend(local_vertices);
                indices.extend(local_indices);
                normals.extend(local_normals);
                uvs.extend(local_uvs);
            }
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices.to_vec());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));

    mesh
}

pub fn world_render_system(
    mut world_map: ResMut<WorldMap>,
    material_resource: Res<MaterialResource>,
    mut commands: Commands,
    mut ev_render: EventReader<WorldRenderRequestUpdateEvent>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for _ in ev_render.read() {
        let mut cloned_map = world_map.clone();
        for (chunk_pos, chunk) in cloned_map.map.iter_mut() {
            match chunk.entity {
                Some(_) => {
                    println!("skip {}", chunk_pos);
                }
                None => update_chunk(
                    chunk_pos,
                    &material_resource,
                    &mut commands,
                    &mut meshes,
                    &mut world_map,
                ),
            };
        }
    }
}

fn update_chunk(
    chunk_pos: &IVec3,
    material_resource: &Res<MaterialResource>,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    world_map: &mut WorldMap,
) {
    let texture = match material_resource.block_materials.get(&Block::Grass) {
        Some(t) => t,
        None => return,
    };
    println!("update_chunk {}", chunk_pos);
    let mesh = generate_chunk_mesh(world_map, chunk_pos);

    // Cube
    let new_entity = commands
        .spawn(PbrBundle {
            mesh: meshes.add(mesh),
            material: texture.clone(),
            transform: Transform::from_xyz(
                (chunk_pos.x * CHUNK_SIZE) as f32,
                (chunk_pos.y * CHUNK_SIZE) as f32,
                (chunk_pos.z * CHUNK_SIZE) as f32,
            ),
            ..Default::default()
        })
        .id();

    let mut ch = world_map.map.get_mut(&chunk_pos).unwrap();
    ch.entity = Some(new_entity);
}
