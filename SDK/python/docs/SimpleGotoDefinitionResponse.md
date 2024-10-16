# SimpleGotoDefinitionResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**definitions** | [**List[SimpleLocation]**](SimpleLocation.md) |  | 
**raw_response** | **object** |  | 

## Example

```python
from openapi_client.models.simple_goto_definition_response import SimpleGotoDefinitionResponse

# TODO update the JSON string below
json = "{}"
# create an instance of SimpleGotoDefinitionResponse from a JSON string
simple_goto_definition_response_instance = SimpleGotoDefinitionResponse.from_json(json)
# print the JSON string representation of the object
print(SimpleGotoDefinitionResponse.to_json())

# convert the object into a dict
simple_goto_definition_response_dict = simple_goto_definition_response_instance.to_dict()
# create an instance of SimpleGotoDefinitionResponse from a dict
simple_goto_definition_response_from_dict = SimpleGotoDefinitionResponse.from_dict(simple_goto_definition_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


