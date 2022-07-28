#version 450

layout(push_constant) uniform PushConstantData {
    vec2 mousePos;
    vec2 resolution;
} pc;

layout(location = 0) out vec4 f_color;


void main() {
    vec2 pixelPos = gl_FragCoord.xy/pc.resolution;
    vec3 baseColor = vec3(1., 0.1, 0.4);
    float distMin = 1. - distance(pixelPos, pc.mousePos);
    float dropOff = pow(distMin * 2, 3);
    vec3 col = baseColor * dropOff;
    f_color = vec4(col, 1.);
}