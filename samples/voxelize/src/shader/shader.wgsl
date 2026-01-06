struct CameraUniform {
	proj_view: mat4x4<f32>,
};
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct MeshUniform {
	trans: mat4x4<f32>,
};
@group(1) @binding(0)
var<uniform> mesh: MeshUniform;

@group(2) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(2) @binding(1)
var s_diffuse: sampler;

// FIX :: Generate this from primitive vertex attributes
struct VertexInput {
	@location(0) position: vec3<f32>,
	@location(1) normal: vec3<f32>,
	@location(2) uv: vec2<f32>
};

struct VertexOutput {
	@builtin(position) clip_position: vec4<f32>,
	@location(0) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(
	model: VertexInput
) -> VertexOutput {
	var out: VertexOutput;
	// Perspective * View * World * Local
	out.clip_position = camera.proj_view * mesh.trans * vec4<f32>(model.position, 1.0);
	out.tex_coords = model.uv;
	return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	let obj_color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
	return obj_color;
}
