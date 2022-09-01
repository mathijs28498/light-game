#version 450

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInput input_attachment;
layout(location = 0) out vec4 f_color;

void main() {
    vec4 input_col = subpassLoad(input_attachment);
    
    if (length(input_col.xyz) > 0.01) {
        f_color = vec4(1., 1., 1., 0) - input_col;
    } else {
        f_color = input_col;
    }
}
