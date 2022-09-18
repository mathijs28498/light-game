#version 450

layout(set = 0, binding = 0, rgba8) uniform image2D image;
layout(location = 0) out vec4 f_color;

void main() {
    float PI_2 = 6.28318530718; // Pi*2
    // TODO: Get resolution xy via pushconstant
    vec2 resolution = vec2(1280, 720);

    // Play around with these values    
    // GAUSSIAN BLUR SETTINGS {{{
    float directions = 16.0; // BLUR DIRECTIONS (Default 16.0 - More is better but slower)
    float quality = 3.0; // BLUR QUALITY (Default 4.0 - More is better but slower)
    float size = 6.0; // BLUR SIZE (Radius)
    // GAUSSIAN BLUR SETTINGS }}}
    
    vec4 color = imageLoad(image, ivec2(gl_FragCoord.xy));
    for (float d = 0.0; d < PI_2; d += PI_2 / directions) {
		for (float i = 1.0 / quality; i <= 1.0; i += 1.0 / quality) {	
            color += imageLoad(image, ivec2(gl_FragCoord.xy) + ivec2(vec2(cos(d), sin(d)) * size * i) );
        }
    }
    
    color *= 1 / (quality * directions - 15.0);
    f_color = color;
    // imageStore(image, ivec2(gl_FragCoord.xy), f_color);
}