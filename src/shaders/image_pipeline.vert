#version 450

layout(push_constant) uniform PushConstantData {
    vec2 resolution;
    vec2 modelCenter;
    float color_mult;
} pc;

layout(location = 0) in vec2 position;

vec2 worldToScreen(vec2 worldPos);

void main() {
    gl_Position = vec4(worldToScreen(position + pc.modelCenter), 0., 1.0);
}

vec2 worldToScreen(vec2 worldPos) {
    return (worldPos / pc.resolution) * 2. - 1.;
}