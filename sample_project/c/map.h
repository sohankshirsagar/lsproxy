#ifndef MAP_H
#define MAP_H

#include "pathfinding.h"

#define MAP_SIZE_ROWS 10
#define MAP_SIZE_COLS 10

extern char map[MAP_SIZE_ROWS][MAP_SIZE_COLS];
extern int ind[MAP_SIZE_ROWS][MAP_SIZE_COLS];

void print_map(int *path, int p_len, struct stop *stops);

#endif