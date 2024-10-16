# openapi_client.CrateApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**file_symbols**](CrateApi.md#file_symbols) | **GET** /file-symbols | 
[**get_definition**](CrateApi.md#get_definition) | **GET** /definition | 
[**get_references**](CrateApi.md#get_references) | **GET** /references | 
[**workspace_symbols**](CrateApi.md#workspace_symbols) | **GET** /workspace-symbols | 


# **file_symbols**
> SimpleSymbolResponse file_symbols(file_path)



### Example


```python
import openapi_client
from openapi_client.models.simple_symbol_response import SimpleSymbolResponse
from openapi_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = openapi_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with openapi_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = openapi_client.CrateApi(api_client)
    file_path = 'file_path_example' # str | 

    try:
        api_response = api_instance.file_symbols(file_path)
        print("The response of CrateApi->file_symbols:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling CrateApi->file_symbols: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **file_path** | **str**|  | 

### Return type

[**SimpleSymbolResponse**](SimpleSymbolResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Symbols retrieved successfully |  -  |
**400** | Bad request |  -  |
**500** | Internal server error |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **get_definition**
> SimpleGotoDefinitionResponse get_definition(file_path, line, character)



### Example


```python
import openapi_client
from openapi_client.models.simple_goto_definition_response import SimpleGotoDefinitionResponse
from openapi_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = openapi_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with openapi_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = openapi_client.CrateApi(api_client)
    file_path = 'file_path_example' # str | 
    line = 56 # int | 
    character = 56 # int | 

    try:
        api_response = api_instance.get_definition(file_path, line, character)
        print("The response of CrateApi->get_definition:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling CrateApi->get_definition: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **file_path** | **str**|  | 
 **line** | **int**|  | 
 **character** | **int**|  | 

### Return type

[**SimpleGotoDefinitionResponse**](SimpleGotoDefinitionResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Definition retrieved successfully |  -  |
**400** | Bad request |  -  |
**500** | Internal server error |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **get_references**
> SimpleReferenceResponse get_references(file_path, line, character, include_declaration=include_declaration)



### Example


```python
import openapi_client
from openapi_client.models.simple_reference_response import SimpleReferenceResponse
from openapi_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = openapi_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with openapi_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = openapi_client.CrateApi(api_client)
    file_path = 'file_path_example' # str | 
    line = 56 # int | 
    character = 56 # int | 
    include_declaration = True # bool |  (optional)

    try:
        api_response = api_instance.get_references(file_path, line, character, include_declaration=include_declaration)
        print("The response of CrateApi->get_references:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling CrateApi->get_references: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **file_path** | **str**|  | 
 **line** | **int**|  | 
 **character** | **int**|  | 
 **include_declaration** | **bool**|  | [optional] 

### Return type

[**SimpleReferenceResponse**](SimpleReferenceResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | References retrieved successfully |  -  |
**400** | Bad request |  -  |
**500** | Internal server error |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

# **workspace_symbols**
> SimpleSymbolResponse workspace_symbols(query)



### Example


```python
import openapi_client
from openapi_client.models.simple_symbol_response import SimpleSymbolResponse
from openapi_client.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = openapi_client.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with openapi_client.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = openapi_client.CrateApi(api_client)
    query = 'query_example' # str | 

    try:
        api_response = api_instance.workspace_symbols(query)
        print("The response of CrateApi->workspace_symbols:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling CrateApi->workspace_symbols: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **query** | **str**|  | 

### Return type

[**SimpleSymbolResponse**](SimpleSymbolResponse.md)

### Authorization

No authorization required

### HTTP request headers

 - **Content-Type**: Not defined
 - **Accept**: application/json

### HTTP response details

| Status code | Description | Response headers |
|-------------|-------------|------------------|
**200** | Workspace symbols retrieved successfully |  -  |
**400** | Bad request |  -  |
**500** | Internal server error |  -  |

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

