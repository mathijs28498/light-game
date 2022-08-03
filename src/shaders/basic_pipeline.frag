#version 450

struct Light
{
    vec2 center;
    float radius;
    float brightness;
    vec3 color;
    float fillerData;
};

layout(set = 0, binding = 0) buffer Data {
    Light lights[];
} data;

layout(push_constant) uniform PushConstantData {
    vec2 mousePos;
    vec2 resolution;
    vec2 dimensions;
    float timePassed;
    uint amountOfLights;
} pc;

layout(location = 0) out vec4 f_color;

float map(float value, float min1, float max1, float min2, float max2);
vec3 getLightColor(vec3 baseColor, float brightnessFactor, vec2 lightPos, float radius);

void main() {
    vec3 col = vec3(0.);

    for (uint i = 0; i < pc.amountOfLights; i++) {
        Light light = data.lights[i];
        col += getLightColor(light.color, light.brightness, light.center, light.radius);
    }
    
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
