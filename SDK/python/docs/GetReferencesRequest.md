# GetReferencesRequest

Request to get the references of a symbol in the workspace.

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**include_declaration** | **bool** | Whether to include the declaration (definition) of the symbol in the response. Defaults to false. | [optional] 
**include_raw_response** | **bool** | Whether to include the raw response from the langserver in the response. Defaults to false. | [optional] 
**symbol_identifier_position** | [**FilePosition**](FilePosition.md) |  | 

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


