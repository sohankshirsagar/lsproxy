#ifndef PATHFINDING_H
#define PATHFINDING_H

struct stop {
    double col, row;
    int *n;
    int n_len;
    double f, g, h;
    int from;
};

struct route {
    int x;
    int y;
    double d;
};

int find_path(struct stop *stops, struct route *routes, int s_len, int *path, int *p_len);
void init_stops(struct stop **stops, int *s_len);
void init_routes(struct route **routes, int *r_len, struct stop *stops, int s_len);

#endif