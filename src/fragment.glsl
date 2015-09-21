#version 140

in vec2 v_tex_coords;
out vec4 color;

uniform sampler2D tex;

void main() {
    float a = texture(tex, v_tex_coords).r;
    color = vec4(a, a, a, 1.0);
}
