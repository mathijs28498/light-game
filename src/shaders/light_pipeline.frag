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

float map(float value, float min1, float max1, float min2, float max2);
vec3 getLightColor(vec3 baseColor, float brightnessFactor, vec2 lightPos, float radius);

void main() {
    vec4 input_col = subpassLoad(input_attachment);
    
    vec3 col = getLightColor(pc.lightColor, pc.lightBrightness, pc.lightCenter, pc.lightRadius);
    
    f_color = input_col + vec4(col, 1.);
}

vec3 getLightColor(vec3 baseColor, float brightnessFactor, vec2 lightPos, float radius) {
    float pixelBrightness = map(radius - distance(gl_FragCoord.xy, lightPos), 0., radius, 0., 1.);
    if (pixelBrightness < 0.) {
        pixelBrightness = 0.;
        // pixelBrightness = 1.;
    }

    pixelBrightness *= brightnessFactor;
    if (pixelBrightness > 1.) {
        pixelBrightness = pow(pixelBrightness, 2);
    } 
    

    return baseColor * pixelBrightness;

}

float map(float value, float min1, float max1, float min2, float max2) {
  return min2 + (value - min1) * (max2 - min2) / (max1 - min1);
}
