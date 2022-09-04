#version 450

layout(push_constant) uniform PushConstantData {
    vec2 resolution;
    vec2 modelCenter;
    vec3 modelColor;
    vec2 cameraPosition;
} pc;

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput input_attachment;
layout(location = 0) out vec4 f_color;

void main() {
    vec4 input_col = subpassLoad(input_attachment);

    f_color = input_col * vec4(pc.modelColor, 1.);
}
