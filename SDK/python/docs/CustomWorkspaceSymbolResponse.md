# CustomWorkspaceSymbolResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**raw_response** | **object** |  | 
**workspace_symbols** | [**List[SimplifiedWorkspaceSymbol]**](SimplifiedWorkspaceSymbol.md) |  | 

## Example

```python
from openapi_client.models.custom_workspace_symbol_response import CustomWorkspaceSymbolResponse

# TODO update the JSON string below
json = "{}"
# create an instance of CustomWorkspaceSymbolResponse from a JSON string
custom_workspace_symbol_response_instance = CustomWorkspaceSymbolResponse.from_json(json)
# print the JSON string representation of the object
print(CustomWorkspaceSymbolResponse.to_json())

# convert the object into a dict
custom_workspace_symbol_response_dict = custom_workspace_symbol_response_instance.to_dict()
# create an instance of CustomWorkspaceSymbolResponse from a dict
custom_workspace_symbol_response_from_dict = CustomWorkspaceSymbolResponse.from_dict(custom_workspace_symbol_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


