# GetReferencesRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**character** | **int** |  | 
**file_path** | **str** |  | 
**include_declaration** | **bool** |  | [optional] 
**line** | **int** |  | 

## Example

```python
from lsproxy_sdk.models.get_references_request import GetReferencesRequest

# TODO update the JSON string below
json = "{}"
# create an instance of GetReferencesRequest from a JSON string
get_references_request_instance = GetReferencesRequest.from_json(json)
# print the JSON string representation of the object
print(GetReferencesRequest.to_json())

# convert the object into a dict
get_references_request_dict = get_references_request_instance.to_dict()
# create an instance of GetReferencesRequest from a dict
get_references_request_from_dict = GetReferencesRequest.from_dict(get_references_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


