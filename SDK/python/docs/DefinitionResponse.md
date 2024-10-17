# DefinitionResponse

Response to a definition request.

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**definitions** | [**List[FilePosition]**](FilePosition.md) | The definition(s) of the symbol. Points to the start position of the symbol&#39;s identifier.  e.g. for the definition of &#x60;User&#x60; on line 5 of &#x60;src/main.py&#x60; with the code: &#x60;&#x60;&#x60; 0: class User: _________^ 1:     def __init__(self, name, age): 2:         self.name &#x3D; name 3:         self.age &#x3D; age 4: 5: user &#x3D; User(\&quot;John\&quot;, 30) __________^ &#x60;&#x60;&#x60; The definition(s) will be &#x60;[{\&quot;path\&quot;: \&quot;src/main.py\&quot;, \&quot;line\&quot;: 0, \&quot;character\&quot;: 6}]&#x60;. | 
**raw_response** | **object** | The raw response from the langserver.  https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_definition | [optional] 

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


