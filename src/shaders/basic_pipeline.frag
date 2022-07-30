#version 450

layout(push_constant) uniform PushConstantData {
    vec2 mousePos;
    vec2 resolution;
    vec2 dimensions;
    float timePassed;
} pc;

layout(location = 0) out vec4 f_color;

float map(float value, float min1, float max1, float min2, float max2);
vec3 getLightColor(vec3 baseColor, float brightnessFactor, vec2 lightPos, float radius);

void main() {
    vec3 baseColor = vec3(0.2, 0.1, 0.7);
    float brightnessFactor = 6.;
    
    vec2 lightPos = pc.mousePos;
    float smallestDim = min(pc.dimensions.x, pc.dimensions.y);
    float radius = 700.;


    vec3 col = getLightColor(baseColor, brightnessFactor, lightPos, radius);
    
    f_color = vec4(col, 1.);
}

vec3 getLightColor(vec3 baseColor, float brightnessFactor, vec2 lightPos, float radius) {
    float pixelBrightness = map(radius - distance(gl_FragCoord.xy, lightPos), 0., radius, 0., 1.);
    if (pixelBrightness < 0.) 
        pixelBrightness = 0.;

    pixelBrightness = pixelBrightness * brightnessFactor;

    return baseColor * pixelBrightness;

}

float map(float value, float min1, float max1, float min2, float max2) {
  return min2 + (value - min1) * (max2 - min2) / (max1 - min1);
}
