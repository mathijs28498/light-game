#version 450

layout(push_constant) uniform PushConstantData {
    vec2 mousePos;
    vec2 resolution;
    vec2 dimensions;
    float timePassed;
    float amountOfLights;
} pc;

layout(location = 0) in vec3 position;

vec2 worldToScreen(vec2 worldPos);

void main() {
    gl_Position = vec4(worldToScreen(position.xy), position.z, 1.0);
}

vec2 worldToScreen(vec2 worldPos) {
    return (worldPos / pc.dimensions) * 2. - 1.;
}