import requests
import json
import argparse
import sys
from typing import Dict, Any, Optional

import openapi_client
from openapi_client.rest import ApiException

BASE_URL = "http://localhost:8080"  # You can change this to an environment variable if needed

configuration = openapi_client.Configuration(
    host = BASE_URL
)

def save_edge_data(data: Dict[str, set], output_file: str = 'edge_data.json'):
    graph_data = [{'from': edge[0], 'to': edge[1], 'referenced_symbols': list(referenced_symbols)} for edge, referenced_symbols in data.items()]
    with open(output_file, 'w') as f:
        json.dump(graph_data, f, indent=2)
    print(f"Dependency data saved to {output_file}")

def process_file(file_path: str):
    with openapi_client.ApiClient(configuration) as api_client:
        api_instance = openapi_client.CrateApi(api_client)
    try:
        edges = {}
        document_symbols = api_instance.file_symbols(file_path).document_symbols

        if document_symbols:
            for symbol in document_symbols:
                name, line, character = symbol.name, symbol.line, symbol.character
                references = api_instance.get_references(file_path, line, character).references
                for reference in references:
                    dest_file = reference.uri
                    if dest_file == file_path:
                        continue
                    edges.setdefault((file_path, dest_file), set()).add(name)
        
        save_edge_data(edges)
    except ApiException as e:
        print("Exception when calling CrateApi->file_symbols: %s\n" % e)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Process file symbols and references using LSP Proxy API.")
    parser.add_argument("file_path", help="Path to the file to be processed")
    args = parser.parse_args()

    process_file(args.file_path)
