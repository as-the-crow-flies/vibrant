struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>
}

struct Node {
    t0: vec3<f32>,
    t1: vec3<f32>,
    index: u32,
    voxel: vec3<u32>
}

const CHILDREN: array<vec3<u32>, 8> = array<vec3<u32>, 8>(
    vec3<u32>(0, 0, 0),
    vec3<u32>(0, 0, 1),
    vec3<u32>(0, 1, 0),
    vec3<u32>(0, 1, 1),
    vec3<u32>(1, 0, 0),
    vec3<u32>(1, 0, 1),
    vec3<u32>(1, 1, 0),
    vec3<u32>(1, 1, 1),
);

const TRANSITIONS: array<vec3<u32>, 8> = array<vec3<u32>, 8>(
    vec3<u32>(4, 2, 1),
    vec3<u32>(5, 3, 8),
    vec3<u32>(6, 8, 3),
    vec3<u32>(7, 8, 8),
    vec3<u32>(8, 6, 5),
    vec3<u32>(8, 7, 8),
    vec3<u32>(8, 8, 7),
    vec3<u32>(8, 8, 8),
);

fn traverse_octree(ray: Ray, octree: texture_3d<f32>) {
    let max_level = i32(textureNumLevels(octree)) - 2;

    var reflection = 0u;
    var origin = ray.origin;
    var direction = ray.direction;

   	if (direction.x < 0.0) {
		origin.x = 1.0 - origin.x;
		direction.x = -direction.x;
		reflection |= 4;
	}
	if (direction.y < 0.0) {
		origin.y = 1.0 - origin.y;
		direction.y = -direction.y;
		reflection |= 2;
	}
	if (direction.z < 0.0) {
		origin.z = 1.0 - origin.z;
		direction.z = -direction.z;
		reflection |= 1;
	}

	let t0_root = (0.0 - origin) / direction;
	let t1_root = (1.0 - origin) / direction;

	if (max(max(t0_root.x, t0_root.y), t0_root.z) >= min(min(t1_root.x, t1_root.y), t1_root.z)) { return; }

	var nodes_counter = 0i;
	var nodes = array<Node, 10>();

	nodes[nodes_counter] = Node(t0_root, t1_root, first_index(t0_root, t1_root), vec3<u32>(0));

	var safety = 0u;

	while (nodes_counter >= 0) {
        safety++;
    	if (safety >= 256) { return; }

	    let node = nodes[nodes_counter];
		let level = max_level - nodes_counter;

		let occupied = textureLoad(octree, node.voxel, level).x > 0.0;

		if (!occupied || node.index == 8 || any(node.t1 < vec3<f32>(0.0))) {
		    nodes_counter--;
		    continue;
		}

		if (level == 0) {
		    if (visit(node)) { return; }
		}

		let mask = vec3<bool>(CHILDREN[node.index]);

		let tm = 0.5 * (node.t0 + node.t1);
		let t0 = select(node.t0, tm, mask);
		let t1 = select(tm, node.t1, mask);

		nodes[nodes_counter].index = next_index(t1, TRANSITIONS[node.index]);

  		nodes_counter++;
  		let child = 2 * node.voxel + CHILDREN[node.index ^ reflection];
  		nodes[nodes_counter] = Node(t0, t1, first_index(t0, t1), child);
	}
}

fn next_index(tm: vec3<f32>, nodes: vec3<u32>) -> u32 {
	if (tm.x < tm.y) {
	    if (tm.x < tm.z) { return nodes.x; } // YZ plane
	} else {
		if (tm.y < tm.z) { return nodes.y; } // XZ plane
	}
	return nodes.z; // XY plane;
}

fn first_index(t0: vec3<f32>, tm: vec3<f32>) -> u32 {
    let maximum = max(max(t0.x, t0.y), t0.z);

	var node = 0u;

	if (maximum == t0.x) { // Entry Plane: YZ
		if(tm.y < t0.x) { node |= 2; }
		if(tm.z < t0.x) { node |= 4; }
	}
	else if (maximum == t0.y) { // Entry Plane: XZ
		if(tm.x < t0.y) { node |= 1; }
		if(tm.z < t0.y) { node |= 4; }
	}
	else if (maximum == t0.z) { // Entry Plane: XY
		if(tm.x < t0.z) { node |= 1; }
		if(tm.y < t0.z) { node |= 2; }
	}

	return node;
}
