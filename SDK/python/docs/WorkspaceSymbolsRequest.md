# WorkspaceSymbolsRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**query** | **str** |  | 

## Example

```python
from lsproxy_sdk.models.workspace_symbols_request import WorkspaceSymbolsRequest

# TODO update the JSON string below
json = "{}"
# create an instance of WorkspaceSymbolsRequest from a JSON string
workspace_symbols_request_instance = WorkspaceSymbolsRequest.from_json(json)
# print the JSON string representation of the object
print(WorkspaceSymbolsRequest.to_json())

# convert the object into a dict
workspace_symbols_request_dict = workspace_symbols_request_instance.to_dict()
# create an instance of WorkspaceSymbolsRequest from a dict
workspace_symbols_request_from_dict = WorkspaceSymbolsRequest.from_dict(workspace_symbols_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


