import requests
import json
import argparse
import sys
from typing import Dict, Any, Optional

from util import SYMBOLKIND_TO_COL_OFFSET

BASE_URL = "http://localhost:8080"  # You can change this to an environment variable if needed

def send_request(endpoint: str, payload: Dict[str, Any]) -> Optional[Dict[str, Any]]:
    """
    Sends a POST request to the specified endpoint of the LSP Proxy API.

    Args:
    endpoint (str): The API endpoint (e.g., "/file-symbols", "/references")
    payload (dict): The request payload

    Returns:
    dict: The JSON response from the server, or None if an error occurred
    """
    url = f"{BASE_URL}{endpoint}"
    headers = {"Content-Type": "application/json"}

    try:
        response = requests.post(url, headers=headers, data=json.dumps(payload))
        response.raise_for_status()
        return response.json()
    except requests.exceptions.RequestException as e:
        print(f"An error occurred: {e}")
        return None

def get_file_symbols(file_path: str) -> Optional[Dict[str, Any]]:
    """Calls the /file-symbols route of the LSP Proxy API."""
    return send_request("/file-symbols", {"file_path": file_path})

def get_references(file_path: str, line: int, character: int, include_declaration: bool = True) -> Optional[Dict[str, Any]]:
    """Calls the /references route of the LSP Proxy API."""
    payload = {
        "file_path": file_path,
        "line": line,
        "character": character,
        "include_declaration": include_declaration
    }
    return send_request("/references", payload)

def save_edge_data(data: Dict[str, set], output_file: str = 'edge_data.json'):
    graph_data = [{'from': edge[0], 'to': edge[1], 'referenced_symbols': list(referenced_symbols)} for edge, referenced_symbols in data.items()]
    with open(output_file, 'w') as f:
        json.dump(graph_data, f, indent=2)
    print(f"Dependency data saved to {output_file}")

def process_file(file_path: str):
    result = get_file_symbols(file_path)
    edges = {}
    
    if result:
        print("Processing file symbols...")
        for el in result:
            name = el["name"]
            line = el["location"]["range"]["start"]["line"]
            character = el["location"]["range"]["start"]["character"]
            character += SYMBOLKIND_TO_COL_OFFSET[el["kind"]]
            references = get_references(file_path, line, character)
            if references:
                for reference in references:
                    dest = reference["uri"].replace("file:///mnt/repo/", "")
                    if dest == file_path:
                        continue
                    edges.setdefault((file_path, dest), set()).add(name)
        
        save_edge_data(edges)
    else:
        print("Failed to retrieve file symbols.")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Process file symbols and references using LSP Proxy API.")
    parser.add_argument("file_path", help="Path to the file to be processed")
    args = parser.parse_args()

    process_file(args.file_path)
