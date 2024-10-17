import requests
import json
import argparse
import sys
from typing import Dict, Any, Optional

from lsproxy_sdk import ApiClient, CrateApi, Configuration
from lsproxy_sdk.models import FilePosition
from lsproxy_sdk.rest import ApiException

def save_edge_data(data: Dict[str, set], output_file: str = 'edge_data.json'):
    graph_data = [{'from': edge[0], 'to': edge[1], 'referenced_symbols': list(referenced_symbols)} for edge, referenced_symbols in data.items()]
    with open(output_file, 'w') as f:
        json.dump(graph_data, f, indent=2)
    print(f"Dependency data saved to {output_file}")

def process_file(file_path: str):
    with ApiClient(Configuration(host="http://localhost:4444")) as lsproxy_client:
        lsproxy = CrateApi(lsproxy_client)
    try:
        edges = {}
        symbols = lsproxy.file_symbols(file_path).symbols or [] 

        for symbol in symbols:
            name = symbol.name
            line = symbol.identifier_start_position.line
            character = symbol.identifier_start_position.character
            references = lsproxy.references(FilePosition(path=file_path, line=line, character=character)).references
            for reference in references:
                dest_file = reference.path
                if dest_file == file_path:
                    continue
                print(f"`{dest_file}` references `{name}` from `{file_path}`")
                edges.setdefault((file_path, dest_file), set()).add(name)
        
        save_edge_data(edges)
    except ApiException as e:
        print("Exception when calling CrateApi->file_symbols: %s\n" % e)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Process file symbols and references using LSP Proxy API.")
    parser.add_argument("file_path", help="Path to the file to be processed")
    args = parser.parse_args()

    process_file(args.file_path)
