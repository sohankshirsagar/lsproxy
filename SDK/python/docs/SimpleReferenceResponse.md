# SimpleReferenceResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**raw_response** | **object** |  | 
**references** | [**List[SimpleLocation]**](SimpleLocation.md) |  | 

## Example

```python
from lsproxy_sdk.models.simple_reference_response import SimpleReferenceResponse

# TODO update the JSON string below
json = "{}"
# create an instance of SimpleReferenceResponse from a JSON string
simple_reference_response_instance = SimpleReferenceResponse.from_json(json)
# print the JSON string representation of the object
print(SimpleReferenceResponse.to_json())

# convert the object into a dict
simple_reference_response_dict = simple_reference_response_instance.to_dict()
# create an instance of SimpleReferenceResponse from a dict
simple_reference_response_from_dict = SimpleReferenceResponse.from_dict(simple_reference_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


