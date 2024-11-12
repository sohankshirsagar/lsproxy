#include <stdlib.h>
#include <math.h>
#include <float.h>
#include <iso646.h>
#include "pathfinding.h"
#include "map.h"

void init_stops(struct stop **stops, int *s_len) {
    int i, j;
    *s_len = 0;
    
    for (i = 1; i < MAP_SIZE_ROWS - 1; i++) {
        for (j = 1; j < MAP_SIZE_COLS - 1; j++) {
            if (!map[i][j]) {
                ++(*s_len);
                *stops = realloc(*stops, (*s_len) * sizeof(struct stop));
                int t = *s_len - 1;
                (*stops)[t].col = j;
                (*stops)[t].row = i;
                (*stops)[t].from = -1;
                (*stops)[t].g = DBL_MAX;
                (*stops)[t].n_len = 0;
                (*stops)[t].n = NULL;
                ind[i][j] = t;
            }
        }
    }
    
    // Calculate heuristics
    int e = *s_len - 1;
    for (i = 0; i < *s_len; i++) {
        (*stops)[i].h = sqrt(pow((*stops)[e].row - (*stops)[i].row, 2) + 
                            pow((*stops)[e].col - (*stops)[i].col, 2));
    }
}

void init_routes(struct route **routes, int *r_len, struct stop *stops, int s_len) {
    int i, j, k, l;
    *r_len = 0;
    
    for (i = 1; i < MAP_SIZE_ROWS - 1; i++) {
        for (j = 1; j < MAP_SIZE_COLS - 1; j++) {
            if (ind[i][j] >= 0) {
                for (k = i - 1; k <= i + 1; k++) {
                    for (l = j - 1; l <= j + 1; l++) {
                        if ((k == i) and (l == j)) continue;
                        if (ind[k][l] >= 0) {
                            ++(*r_len);
                            *routes = realloc(*routes, (*r_len) * sizeof(struct route));
                            int t = *r_len - 1;
                            (*routes)[t].x = ind[i][j];
                            (*routes)[t].y = ind[k][l];
                            (*routes)[t].d = sqrt(pow(stops[(*routes)[t].y].row - 
                                                    stops[(*routes)[t].x].row, 2) + 
                                                pow(stops[(*routes)[t].y].col - 
                                                    stops[(*routes)[t].x].col, 2));
                            
                            ++stops[(*routes)[t].x].n_len;
                            stops[(*routes)[t].x].n = realloc(stops[(*routes)[t].x].n, 
                                                            stops[(*routes)[t].x].n_len * sizeof(int));
                            stops[(*routes)[t].x].n[stops[(*routes)[t].x].n_len - 1] = t;
                        }
                    }
                }
            }
        }
    }
}