# CustomGotoDefinitionResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**definitions** | [**List[SimplifiedLocation]**](SimplifiedLocation.md) |  | 
**raw_response** | **object** |  | 

## Example

```python
from openapi_client.models.custom_goto_definition_response import CustomGotoDefinitionResponse

# TODO update the JSON string below
json = "{}"
# create an instance of CustomGotoDefinitionResponse from a JSON string
custom_goto_definition_response_instance = CustomGotoDefinitionResponse.from_json(json)
# print the JSON string representation of the object
print(CustomGotoDefinitionResponse.to_json())

# convert the object into a dict
custom_goto_definition_response_dict = custom_goto_definition_response_instance.to_dict()
# create an instance of CustomGotoDefinitionResponse from a dict
custom_goto_definition_response_from_dict = CustomGotoDefinitionResponse.from_dict(custom_goto_definition_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


