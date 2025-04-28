use bevy::{
    prelude::*,
    render::{mesh::Indices, render_asset::RenderAssetUsages, render_resource::PrimitiveTopology},
};
use noise::{NoiseFn, Perlin};

use crate::track_gen::{BlockType, rotate_point_around};

// Constants
pub const TRACK_WIDTH: f32 = 10.0;
const SEGMENTS_PER_RADIAN: usize = 10;

// Function to generate a mesh for a single block
pub fn generate_mesh_for_block(block: BlockType, noise: &Perlin, offset: Vec3) -> Mesh {
    match block {
        BlockType::Straight { length } => generate_straight_mesh(length),
        BlockType::Turn { angle, radius } => generate_turn_mesh(angle, radius),
        BlockType::Slope {
            length,
            height_change,
        } => generate_slope_mesh(length, height_change),
        BlockType::BankedTurn {
            angle,
            radius,
            bank_height,
        } => generate_banked_turn_mesh(angle, radius, bank_height),
        BlockType::Bumpy {
            length,
            pertubation,
        } => generate_bumpy_mesh(length, TRACK_WIDTH, 20, 20, pertubation, noise, offset), // _ => empty_mesh(),
    }
}

// Straight mesh - a rectangular track segment along the X axis
fn generate_straight_mesh(length: f32) -> Mesh {
    let half_width = TRACK_WIDTH / 2.0;

    // Vertices: 4 corners of the rectangle
    let vertices = vec![
        [-half_width, 0.0, 0.0],    // Bottom left
        [half_width, 0.0, 0.0],     // Bottom right
        [half_width, 0.0, length],  // Top right
        [-half_width, 0.0, length], // Top left
        [-half_width, 3.0, 0.0],    // Bottom left
        [half_width, 3.0, 0.0],     // Bottom right
        [half_width, 3.0, length],  // Top right
        [-half_width, 3.0, length], // Top left
    ];

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
    let uvs = vec![
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
        [0.0, 1.0],
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
        [0.0, 1.0],
    ];
    // Normals: all pointing up (Y+)
    let normals = vec![
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
    ];

    create_mesh_from_attributes(vertices, indices, uvs, normals)
}
fn generate_bumpy_mesh(
    length: f32,
    width: f32,
    resolution_x: usize,
    resolution_z: usize,
    perturbations: f32,
    noise: &Perlin,
    offset: Vec3,
) -> Mesh {
    // --- Validate Input ---
    // Ensure resolution is at least 2x2 to form a grid
    assert!(resolution_x >= 2, "resolution_x must be at least 2");
    assert!(resolution_z >= 2, "resolution_z must be at least 2");

    // --- Initialize Data Structures ---
    let num_vertices = resolution_x * resolution_z;
    let num_quads = (resolution_x - 1) * (resolution_z - 1);
    let num_indices = num_quads * 6; // 2 triangles per quad, 3 indices per triangle

    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(num_vertices);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(num_vertices);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(num_vertices);
    let mut indices: Vec<u32> = Vec::with_capacity(num_indices);

    // --- Generate Vertices, Normals, and UVs ---
    let half_width = width / 2.0;

    for z_idx in 0..resolution_z {
        // Calculate the z-coordinate, mapping the index to the world length
        let z_pos = length * (z_idx as f32) / (resolution_z - 1) as f32;
        // Calculate the v texture coordinate
        let v = z_idx as f32 / (resolution_z - 1) as f32;

        for x_idx in 0..resolution_x {
            // Calculate the x-coordinate, mapping the index to the world width centered at 0
            let x_pos = -half_width + width * (x_idx as f32) / (resolution_x - 1) as f32;
            // Calculate the u texture coordinate
            let u = x_idx as f32 / (resolution_x - 1) as f32;

            // --- Calculate Height Offset ---
            // Apply noise only to inner vertices to keep the edges flat at y=0
            let height =
                if x_idx > 0 && x_idx < resolution_x - 1 && z_idx > 0 && z_idx < resolution_z - 1 {
                    // Use noise function. Note: The `noise` crate often uses f64.
                    // The noise value is typically between -1.0 and 1.0.
                    let noise_val = noise.get([
                        x_pos as f64 / 6.0 + offset.x as f64,
                        z_pos as f64 / 6.0 + offset.y as f64,
                    ]);
                    noise_val as f32 * perturbations
                } else {
                    // Keep border vertices flat
                    0.0
                };

            positions.push([x_pos, height, z_pos]);
            uvs.push([u, v]);
            // Initialize normals pointing up. We'll calculate accurate normals later.
            normals.push([0.0, 1.0, 0.0]);
        }
    }
    positions.extend_from_slice(&[
        [-half_width, 3.0, 0.0],    // Bottom left
        [half_width, 3.0, 0.0],     // Bottom right
        [half_width, 3.0, length],  // Top right
        [-half_width, 3.0, length], // Top left
    ]);
    normals.extend_from_slice(&[
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
    ]);

    // --- Generate Indices ---
    // Iterate through each quad of the grid
    for z_idx in 0..(resolution_z - 1) {
        for x_idx in 0..(resolution_x - 1) {
            // Calculate the indices of the four corners of the current quad
            // The vertices are stored row by row (based on z_idx first, then x_idx)
            let bottom_left = (z_idx * resolution_x + x_idx) as u32;
            let bottom_right = bottom_left + 1;
            let top_left = ((z_idx + 1) * resolution_x + x_idx) as u32;
            let top_right = top_left + 1;

            // Add indices for the first triangle (bottom-left, top-left, bottom-right)
            indices.push(bottom_left);
            indices.push(top_left);
            indices.push(bottom_right);

            // Add indices for the second triangle (bottom-right, top-left, top-right)
            indices.push(bottom_right);
            indices.push(top_left);
            indices.push(top_right);
        }
    }
    // 7 = positions.len()
    // 6 = positions.len() - 1
    // 5 = positions.len() - 2
    // 4 = positions.len() - 3

    indices.extend(vec![
        0,
        positions.len() as u32 - 4,
        positions.len() as u32 - 1, // top left
        0,
        positions.len() as u32 - 1,
        (resolution_x * (resolution_z - 1)) as u32, // bottom left
        (resolution_z as u32 - 0) * (resolution_x as u32 - 0) - 1,
        positions.len() as u32 - 2,
        positions.len() as u32 - 3, // top right
        resolution_x as u32 - 1,
        ((resolution_x - 0) * (resolution_z - 0)) as u32 - 1,
        positions.len() as u32 - 3, // bottom right
    ]);
    // --- Calculate Accurate Normals ---
    // Reset normals to zero before accumulating face normals
    for n in normals.iter_mut() {
        *n = [0.0, 0.0, 0.0];
    }
    normals.push([0.0, 0.0, 0.0]);
    normals.push([0.0, 0.0, 0.0]);
    normals.push([0.0, 0.0, 0.0]);
    normals.push([0.0, 0.0, 0.0]);

    uvs.extend_from_slice(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);

    // --- Create Bevy Mesh ---
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh.compute_smooth_normals();

    mesh
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
        let outer = rotate_point_around(
            Vec2::new(TRACK_WIDTH / 2.0, 0.0),
            Vec2::new(radius, 0.0),
            -segment_angle,
        );

        vertices.push([inner.x, 0.0, inner.y]);
        vertices.push([inner.x, 3.0, inner.y]);
        vertices.push([outer.x, 0.0, outer.y]);
        vertices.push([outer.x, 3.0, outer.y]);
        uvs.push([i as f32 / segments as f32, 0.0]);
        uvs.push([i as f32 / segments as f32, 0.0]);
        uvs.push([i as f32 / segments as f32, 1.0]);
        uvs.push([i as f32 / segments as f32, 1.0]);
        normals.push([0.0, 1.0, 0.0]);
        normals.push([1.0, 0.0, 0.0]);
        normals.push([0.0, 1.0, 0.0]);
        normals.push([-1.0, 0.0, 0.0]);

        // Add indices for the quad (two triangles)
        if i < segments {
            let base_index = i * 4;
            indices.push(base_index + 0); // Current inner floor vertex
            indices.push(base_index + 4); // Next inner floor vertex
            indices.push(base_index + 2); // Current outer floor vertex

            indices.push(base_index + 2); // Current outer floor vertex
            indices.push(base_index + 4); // Next inner floor vertex
            indices.push(base_index + 6); // Next outer floor vertex

            indices.push(base_index + 0); // Current inner floor
            indices.push(base_index + 1); // Current inner ceiling
            indices.push(base_index + 4); // Next inner floor

            indices.push(base_index + 1); // Current inner ceiling
            indices.push(base_index + 5); // Next inner ceiling
            indices.push(base_index + 4); // Next inner floor

            indices.push(base_index + 2); // Current outer floor
            indices.push(base_index + 6); // Next outer floor
            indices.push(base_index + 3); // Current outer ceiling

            indices.push(base_index + 3); // Current outer ceiling
            indices.push(base_index + 6); // Next outer floor
            indices.push(base_index + 7); // Next outer ceiling
        }
    }

    create_mesh_from_attributes(vertices, indices, uvs, normals)
}

// Turn mesh - an arc segment with specified radius and angle
fn generate_banked_turn_mesh(angle: f32, radius: f32, bank_height: f32) -> Mesh {
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
        let outer = rotate_point_around(
            Vec2::new(TRACK_WIDTH / 2.0, 0.0),
            Vec2::new(radius, 0.0),
            -segment_angle,
        );

        let bank_offset = sigmoid_peak(i as usize, segments as usize) * bank_height;
        vertices.push([inner.x, 0.0 + bank_offset, inner.y]);
        vertices.push([inner.x, 3.0 + bank_offset, inner.y]);
        vertices.push([outer.x, 0.0, outer.y]);
        vertices.push([outer.x, 3.0, outer.y]);
        uvs.push([i as f32 / segments as f32, 0.0]);
        uvs.push([i as f32 / segments as f32, 0.0]);
        uvs.push([i as f32 / segments as f32, 1.0]);
        uvs.push([i as f32 / segments as f32, 1.0]);
        normals.push([0.0, 1.0, 0.0]);
        normals.push([1.0, 0.0, 0.0]);
        normals.push([0.0, 1.0, 0.0]);
        normals.push([-1.0, 0.0, 0.0]);

        // Add indices for the quad (two triangles)
        if i < segments {
            let base_index = i * 4;
            indices.push(base_index + 0); // Current inner floor vertex
            indices.push(base_index + 4); // Next inner floor vertex
            indices.push(base_index + 2); // Current outer floor vertex

            indices.push(base_index + 2); // Current outer floor vertex
            indices.push(base_index + 4); // Next inner floor vertex
            indices.push(base_index + 6); // Next outer floor vertex

            indices.push(base_index + 0); // Current inner floor
            indices.push(base_index + 1); // Current inner ceiling
            indices.push(base_index + 4); // Next inner floor

            indices.push(base_index + 1); // Current inner ceiling
            indices.push(base_index + 5); // Next inner ceiling
            indices.push(base_index + 4); // Next inner floor

            indices.push(base_index + 2); // Current outer floor
            indices.push(base_index + 6); // Next outer floor
            indices.push(base_index + 3); // Current outer ceiling

            indices.push(base_index + 3); // Current outer ceiling
            indices.push(base_index + 6); // Next outer floor
            indices.push(base_index + 7); // Next outer ceiling
        }
    }

    create_mesh_from_attributes(vertices, indices, uvs, normals)
}

fn sigmoid_peak(i: usize, max: usize) -> f32 {
    if i == 0 {
        return 0.0;
    }
    if i == max {
        return 0.0;
    }

    let max_f32 = max as f32;
    let half_max = max_f32 / 2.0;
    let i_f32 = i as f32;

    zero_peak((i_f32 - half_max) / half_max)
}

fn zero_peak(x: f32) -> f32 {
    // Calculate x^2 * 5
    // We use powi(2) for integer power, which can be more efficient than powf(2.0)
    // Alternatively, you could just write x * x
    let exponent_term = x.powi(2) * 5.0;

    // Calculate e^(exponent_term) using the exp() method
    let exp_value = exponent_term.exp();

    // Calculate 1 + e^(...)
    let denominator = 1.0 + exp_value;

    // Calculate 1 / (1 + e^(...))
    let fraction = 1.0 / denominator;

    // Calculate the final result * 2
    let result = fraction * 2.0;

    result // Return the result
}

// Slope mesh - a straight segment that changes height
fn generate_slope_mesh(length: f32, height_change: f32) -> Mesh {
    let half_width = TRACK_WIDTH / 2.0;

    // Vertices: 4 corners of the rectangle
    let vertices = vec![
        [-half_width, 0.0, 0.0],                    // Start, left
        [half_width, 0.0, 0.0],                     // Start, right
        [half_width, height_change, length],        // End, right
        [-half_width, height_change, length],       // End, left
        [-half_width, 3.0, 0.0],                    // Bottom left
        [half_width, 3.0, 0.0],                     // Bottom right
        [half_width, height_change + 3.0, length],  // Top right
        [-half_width, height_change + 3.0, length], // Top left
    ];

    // Indices: 2 triangles forming a quad
    let indices = vec![
        0, 2, 1, // First triangle
        0, 3, 2, // Second triangle
        0, 4, 7, // top left
        0, 7, 3, // bottom left
        1, 6, 5, // top right
        1, 2, 6, // bottom right
    ];

    // UVs: simple mapping for a rectangle
    let uvs = vec![
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
        [0.0, 1.0],
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
        [0.0, 1.0],
    ];

    // Calculate normalized normal for the slope
    let dx = length;
    let dy = height_change;
    let normal_length = (dx * dx + dy * dy).sqrt();

    let normal = [
        -dy / normal_length, // X component (depends on slope)
        dx / normal_length,  // Y component (depends on slope)
        0.0,                 // Z component (no tilt in Z direction)
    ];

    let normals = vec![
        normal,
        normal,
        normal,
        normal,
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
    ];

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
