# lsproxy_sdk.CrateApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**definition**](CrateApi.md#definition) | **GET** /definition | 
[**file_symbols**](CrateApi.md#file_symbols) | **GET** /file-symbols | 
[**references**](CrateApi.md#references) | **GET** /references | 
[**workspace_symbols**](CrateApi.md#workspace_symbols) | **GET** /workspace-symbols | 


# **definition**
> DefinitionResponse definition(position)



### Example


```python
import lsproxy_sdk
from lsproxy_sdk.models.definition_response import DefinitionResponse
from lsproxy_sdk.models.file_position import FilePosition
from lsproxy_sdk.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = lsproxy_sdk.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with lsproxy_sdk.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = lsproxy_sdk.CrateApi(api_client)
    position = lsproxy_sdk.FilePosition() # FilePosition | 

    try:
        api_response = api_instance.definition(position)
        print("The response of CrateApi->definition:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling CrateApi->definition: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **position** | [**FilePosition**](.md)|  | 

### Return type

[**DefinitionResponse**](DefinitionResponse.md)

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

# **file_symbols**
> SymbolResponse file_symbols(file_path)



### Example


```python
import lsproxy_sdk
from lsproxy_sdk.models.symbol_response import SymbolResponse
from lsproxy_sdk.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = lsproxy_sdk.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with lsproxy_sdk.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = lsproxy_sdk.CrateApi(api_client)
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

[**SymbolResponse**](SymbolResponse.md)

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

# **references**
> ReferenceResponse references(symbol_identifier_position, include_declaration=include_declaration)



### Example


```python
import lsproxy_sdk
from lsproxy_sdk.models.file_position import FilePosition
from lsproxy_sdk.models.reference_response import ReferenceResponse
from lsproxy_sdk.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = lsproxy_sdk.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with lsproxy_sdk.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = lsproxy_sdk.CrateApi(api_client)
    symbol_identifier_position = lsproxy_sdk.FilePosition() # FilePosition | 
    include_declaration = True # bool |  (optional)

    try:
        api_response = api_instance.references(symbol_identifier_position, include_declaration=include_declaration)
        print("The response of CrateApi->references:\n")
        pprint(api_response)
    except Exception as e:
        print("Exception when calling CrateApi->references: %s\n" % e)
```



### Parameters


Name | Type | Description  | Notes
------------- | ------------- | ------------- | -------------
 **symbol_identifier_position** | [**FilePosition**](.md)|  | 
 **include_declaration** | **bool**|  | [optional] 

### Return type

[**ReferenceResponse**](ReferenceResponse.md)

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
> SymbolResponse workspace_symbols(query)



### Example


```python
import lsproxy_sdk
from lsproxy_sdk.models.symbol_response import SymbolResponse
from lsproxy_sdk.rest import ApiException
from pprint import pprint

# Defining the host is optional and defaults to http://localhost
# See configuration.py for a list of all supported configuration parameters.
configuration = lsproxy_sdk.Configuration(
    host = "http://localhost"
)


# Enter a context with an instance of the API client
with lsproxy_sdk.ApiClient(configuration) as api_client:
    # Create an instance of the API class
    api_instance = lsproxy_sdk.CrateApi(api_client)
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

[**SymbolResponse**](SymbolResponse.md)

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

