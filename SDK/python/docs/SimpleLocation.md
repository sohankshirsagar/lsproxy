# SimpleLocation


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**identifier_start_character** | **int** |  | 
**identifier_start_line** | **int** |  | 
**uri** | **str** |  | 

## Example

```python
from openapi_client.models.simple_location import SimpleLocation

# TODO update the JSON string below
json = "{}"
# create an instance of SimpleLocation from a JSON string
simple_location_instance = SimpleLocation.from_json(json)
# print the JSON string representation of the object
print(SimpleLocation.to_json())

# convert the object into a dict
simple_location_dict = simple_location_instance.to_dict()
# create an instance of SimpleLocation from a dict
simple_location_from_dict = SimpleLocation.from_dict(simple_location_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


