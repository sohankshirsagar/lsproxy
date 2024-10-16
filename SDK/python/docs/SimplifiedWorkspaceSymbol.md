# SimplifiedWorkspaceSymbol


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**character** | **int** |  | 
**kind** | **str** |  | 
**line** | **int** |  | 
**name** | **str** |  | 
**uri** | **str** |  | 

## Example

```python
from openapi_client.models.simplified_workspace_symbol import SimplifiedWorkspaceSymbol

# TODO update the JSON string below
json = "{}"
# create an instance of SimplifiedWorkspaceSymbol from a JSON string
simplified_workspace_symbol_instance = SimplifiedWorkspaceSymbol.from_json(json)
# print the JSON string representation of the object
print(SimplifiedWorkspaceSymbol.to_json())

# convert the object into a dict
simplified_workspace_symbol_dict = simplified_workspace_symbol_instance.to_dict()
# create an instance of SimplifiedWorkspaceSymbol from a dict
simplified_workspace_symbol_from_dict = SimplifiedWorkspaceSymbol.from_dict(simplified_workspace_symbol_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


