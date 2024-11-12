#include <stdio.h>
#include <stdlib.h>
#include "map.h"
#include "pathfinding.h"

int main() {
    struct stop *stops = NULL;
    struct route *routes = NULL;
    int s_len = 0, r_len = 0, p_len = 0;
    int *path = NULL;
    
    init_stops(&stops, &s_len);
    init_routes(&routes, &r_len, stops, s_len);
    
    if (find_path(stops, routes, s_len, path, &p_len)) {
        print_map(path, p_len, stops);
        printf("path cost is %d:\n", p_len);
        for (int i = p_len - 1; i >= 0; i--) {
            printf("(%1.0f, %1.0f)\n", stops[path[i]].col, stops[path[i]].row);
        }
    } else {
        puts("IMPOSSIBLE");
    }

    // Cleanup
    for (int i = 0; i < s_len; ++i) {
        free(stops[i].n);
    }
    free(stops);
    free(routes);
    free(path);
    
    return 0;
}