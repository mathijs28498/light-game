#version 450

layout(set = 0, binding = 0, rgba8) uniform readonly image2D img;
layout(location = 0) out vec4 f_color;

void main() {
    // TODO: Add gaussian blur
    vec4 color = vec4(0.);
    int xDiff = 2;
    int yDiff = 2;

    for (int x = -xDiff; x <= xDiff; x++) {
        for (int y = -yDiff; y <= yDiff; y++) {
            color += imageLoad(img, ivec2(gl_FragCoord.xy) + ivec2(x, y));
        }
    }

    color *= 1. / ((xDiff * 2. + 1.) * (yDiff * 2. + 1.));
    f_color = color;
    // f_color = vec4(1.) - color;
}