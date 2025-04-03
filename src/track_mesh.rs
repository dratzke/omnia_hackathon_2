use std::f32::consts::PI;

use bevy::{
    prelude::*,
    render::{mesh::Indices, render_asset::RenderAssetUsages, render_resource::PrimitiveTopology},
};

use crate::track_gen::{BlockType, rotate_point_around};

// Constants
const TRACK_WIDTH: f32 = 10.0;
const SEGMENTS_PER_RADIAN: usize = 10;

// Function to generate a mesh for a single block
pub fn generate_mesh_for_block(block: BlockType) -> Mesh {
    match block {
        BlockType::Straight { length } => generate_straight_mesh(length),
        BlockType::Turn { angle, radius } => generate_turn_mesh(angle, radius),
        BlockType::Slope {
            length,
            height_change,
        } => generate_slope_mesh(length, height_change),
        // _ => empty_mesh(),
    }
}

fn empty_mesh() -> Mesh {
    create_mesh_from_attributes(vec![], vec![], vec![], vec![])
}

// Straight mesh - a rectangular track segment along the X axis
fn generate_straight_mesh(length: f32) -> Mesh {
    let half_width = TRACK_WIDTH / 2.0;

    // Vertices: 4 corners of the rectangle
    let mut vertices = vec![
        [-half_width, 0.0, 0.0],    // Bottom left
        [half_width, 0.0, 0.0],     // Bottom right
        [half_width, 0.0, length],  // Top right
        [-half_width, 0.0, length], // Top left
    ];

    vertices.extend(vec![
        [-half_width, 3.0, 0.0],    // Bottom left
        [half_width, 3.0, 0.0],     // Bottom right
        [half_width, 3.0, length],  // Top right
        [-half_width, 3.0, length], // Top left
    ]);

    // 4 5
    // 7 6

    // 0 1
    // 3 2

    // Indices: 2 triangles forming a quad
    let mut indices = vec![
        0, 2, 1, // First triangle
        0, 3, 2, // Second triangle
    ];

    indices.extend(vec![
        0, 4, 7, // top left
        0, 7, 3, // bottom left
        1, 6, 5, // top right
        1, 2, 6, // bottom right
    ]);

    // UVs: simple mapping for a rectangle
    let mut uvs = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    uvs.extend(vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
    // Normals: all pointing up (Y+)
    let mut normals = vec![
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
    ];

    normals.extend(vec![
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
    ]);

    create_mesh_from_attributes(vertices, indices, uvs, normals)
}

// Turn mesh - an arc segment with specified radius and angle
fn generate_turn_mesh(angle: f32, radius: f32) -> Mesh {
    // Calculate number of segments based on angle
    let segments = (angle.abs() * SEGMENTS_PER_RADIAN as f32).ceil() as u32;
    let segments = segments.max(1); // At least 1 segment

    let mut vertices = Vec::with_capacity(((segments + 1) * 2) as usize);
    let mut uvs = Vec::with_capacity(((segments + 1) * 2) as usize);
    let mut normals = Vec::with_capacity(((segments + 1) * 2) as usize);
    let mut indices: Vec<u32> = Vec::with_capacity((segments * 6) as usize);

    // Generate vertices along the arc
    for i in 0..=segments {
        let segment_angle = i as f32 / segments as f32 * angle;

        let inner = rotate_point_around(
            Vec2::new(-TRACK_WIDTH / 2.0, 0.0),
            Vec2::new(radius, 0.0),
            -segment_angle,
        );

        vertices.push([inner.x, 0.0, inner.y]);
        uvs.push([i as f32 / segments as f32, 0.0]);
        normals.push([0.0, 1.0, 0.0]);

        let outer = rotate_point_around(
            Vec2::new(TRACK_WIDTH / 2.0, 0.0),
            Vec2::new(radius, 0.0),
            -segment_angle,
        );

        vertices.push([outer.x, 0.0, outer.y]);
        // Outer vertex
        uvs.push([i as f32 / segments as f32, 1.0]);
        normals.push([0.0, 1.0, 0.0]);

        // Add indices for the quad (two triangles)
        if i < segments {
            let base_index = i * 2;
            indices.push(base_index);
            indices.push(base_index + 3);
            indices.push(base_index + 1);

            indices.push(base_index);
            indices.push(base_index + 2);
            indices.push(base_index + 3);
        }
    }

    let _ = dbg!(
        vertices[0..2]
            .into_iter()
            .cloned()
            .collect::<Vec<[f32; 3]>>()
    );
    let _ = dbg!(
        vertices[vertices.len() - 2..]
            .into_iter()
            .cloned()
            .collect::<Vec<[f32; 3]>>()
    );

    create_mesh_from_attributes(vertices, indices, uvs, normals)
}

// Slope mesh - a straight segment that changes height
fn generate_slope_mesh(length: f32, height_change: f32) -> Mesh {
    let half_width = TRACK_WIDTH / 2.0;

    // Vertices: 4 corners of the rectangle
    let vertices = vec![
        [-half_width, 0.0, 0.0],              // Start, left
        [half_width, 0.0, 0.0],               // Start, right
        [half_width, height_change, length],  // End, right
        [-half_width, height_change, length], // End, left
    ];

    // Indices: 2 triangles forming a quad
    let indices = vec![
        0, 2, 1, // First triangle
        0, 3, 2, // Second triangle
    ];

    // UVs: simple mapping for a rectangle
    let uvs = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

    // Calculate normalized normal for the slope
    let dx = length;
    let dy = height_change;
    let normal_length = (dx * dx + dy * dy).sqrt();

    let normal = [
        -dy / normal_length, // X component (depends on slope)
        dx / normal_length,  // Y component (depends on slope)
        0.0,                 // Z component (no tilt in Z direction)
    ];

    let normals = vec![normal, normal, normal, normal];

    create_mesh_from_attributes(vertices, indices, uvs, normals)
}

// Helper function to create a mesh from attributes
fn create_mesh_from_attributes(
    positions: Vec<[f32; 3]>,
    indices: Vec<u32>,
    uvs: Vec<[f32; 2]>,
    normals: Vec<[f32; 3]>,
) -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_indices(Indices::U32(indices))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::Vec3;
    use bevy::render::{mesh::VertexAttributeValues, prelude::Mesh};

    #[test]
    fn test_mesh_vertices_in_bounding_box() {
        // Setup test environment

        // Generate test mesh using your mesh generation function
        let mesh = generate_turn_mesh(0.0, 0.0); // Replace with your actual mesh generator

        // Define bounding box constraints
        let min_bound = Vec3::new(-1.0, -0.5, -1.0);
        let max_bound = Vec3::new(1.0, 0.5, 1.0);

        // Verify vertex positions exist and are in correct format
        let positions = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .expect("Mesh missing position attribute");

        let VertexAttributeValues::Float32x3(positions) = positions else {
            panic!("Position attribute has unexpected format");
        };

        // Check each vertex against bounds
        for position in positions {
            assert!(
                position[0] >= min_bound.x && position[0] <= max_bound.x,
                "X coordinate {} out of bounds",
                position[0]
            );
            assert!(
                position[1] >= min_bound.y && position[1] <= max_bound.y,
                "Y coordinate {} out of bounds",
                position[1]
            );
            assert!(
                position[2] >= min_bound.z && position[2] <= max_bound.z,
                "Z coordinate {} out of bounds",
                position[2]
            );
        }
    }
}
