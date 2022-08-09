#version 450

layout(push_constant) uniform PushConstantData {
    vec2 mousePos;
    vec2 resolution;
    float timePassed;
    float lightRadius;
    vec2 lightCenter;
    vec3 lightColor;
    float lightBrightness;
} pc;


layout(location = 0) in vec2 position;

vec2 worldToScreen(vec2 worldPos);

void main() {
    gl_Position = vec4(worldToScreen(position + pc.lightCenter), 0., 1.0);
}

vec2 worldToScreen(vec2 worldPos) {
    return (worldPos / pc.resolution) * 2. - 1.;
}