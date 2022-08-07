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

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput input_attachment;
layout(location = 0) out vec4 f_color;

void main() {
    vec2 test = pc.lightCenter;
    vec4 input_col = subpassLoad(input_attachment);
    f_color = input_col + vec4(0.3, 0.9, 0.7, 1.0);
}