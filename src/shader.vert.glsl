in vec2 co;
in vec3 color;
in vec2 position;
in float weight;

out vec2 v_uv;

const vec2[4] QUAD_POS = vec2[](
  vec2(-1., 1.),
  vec2( 1., 1.),
  vec2( 1.,  -1.),
  vec2(-1.,  -1.)
);

void main() {
  vec2 p = QUAD_POS[gl_VertexID];

  gl_Position = vec4(co * weight + position, 0., 1.);
  v_uv = p * .5 + .5; // transform the position of the vertex into UV space
}

// in vec2 co;
// in vec3 color;
// in vec2 position;
// in float weight;

// out vec3 v_color;
// out float v_instance_bias;

// void main() {
//   gl_Position = vec4(co * weight + position, 0., 1.);
//   v_color = vec3(1.);
//   v_instance_bias = float(gl_InstanceID);
// }
