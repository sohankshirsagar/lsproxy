# GetDefinitionRequest

Request to get the definition of a symbol at a given position in a file.

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**include_raw_response** | **bool** | Whether to include the raw response from the langserver in the response. Defaults to false. | [optional] 
**position** | [**FilePosition**](FilePosition.md) |  | 

## Example

```python
from lsproxy_sdk.models.get_definition_request import GetDefinitionRequest

# TODO update the JSON string below
json = "{}"
# create an instance of GetDefinitionRequest from a JSON string
get_definition_request_instance = GetDefinitionRequest.from_json(json)
# print the JSON string representation of the object
print(GetDefinitionRequest.to_json())

# convert the object into a dict
get_definition_request_dict = get_definition_request_instance.to_dict()
# create an instance of GetDefinitionRequest from a dict
get_definition_request_from_dict = GetDefinitionRequest.from_dict(get_definition_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


