#version 450

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput input_attachment;
layout(location = 0) out vec4 f_color;

layout(location = 0) in vec3 frag_color;

void main() {
    vec4 input_col = subpassLoad(input_attachment);

    f_color = input_col + vec4(frag_color.xyz, 1.0);
}