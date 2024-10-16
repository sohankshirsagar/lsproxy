# DefinitionResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**definitions** | [**List[FilePosition]**](FilePosition.md) |  | 
**raw_response** | **object** |  | 

## Example

```python
from lsproxy_sdk.models.definition_response import DefinitionResponse

# TODO update the JSON string below
json = "{}"
# create an instance of DefinitionResponse from a JSON string
definition_response_instance = DefinitionResponse.from_json(json)
# print the JSON string representation of the object
print(DefinitionResponse.to_json())

# convert the object into a dict
definition_response_dict = definition_response_instance.to_dict()
# create an instance of DefinitionResponse from a dict
definition_response_from_dict = DefinitionResponse.from_dict(definition_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


