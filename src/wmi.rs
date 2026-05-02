use windows::{
    core::*,
    Win32::System::Com::*,
    Win32::System::Wmi::*,
    Win32::System::Ole::*,
    Win32::System::Rpc::*,
    Win32::System::Variant::*,
};
use crate::errors::Result;
use serde_json::{Value, Map};

pub trait WmiProvider: Send + Sync {
    fn query(&self, query: &str) -> Result<IEnumWbemClassObject>;
    fn list_classes(&self) -> Result<IEnumWbemClassObject>;
    fn get_class(&self, class_name: &str) -> Result<IWbemClassObject>;
}

pub struct WmiClient {
    services: IWbemServices,
}

// IWbemServices is thread-safe as long as COM is initialized.
unsafe impl Send for WmiClient {}
unsafe impl Sync for WmiClient {}

impl WmiProvider for WmiClient {
    fn query(&self, query: &str) -> Result<IEnumWbemClassObject> {
        unsafe {
            let language = BSTR::from("WQL");
            let query = BSTR::from(query);
            Ok(self.services.ExecQuery(
                &language,
                &query,
                WBEM_FLAG_FORWARD_ONLY | WBEM_FLAG_RETURN_IMMEDIATELY,
                None,
            )?)
        }
    }

    fn list_classes(&self) -> Result<IEnumWbemClassObject> {
        unsafe {
            Ok(self.services.CreateClassEnum(
                &BSTR::default(),
                WBEM_FLAG_FORWARD_ONLY | WBEM_FLAG_RETURN_IMMEDIATELY,
                None,
            )?)
        }
    }

    fn get_class(&self, class_name: &str) -> Result<IWbemClassObject> {
        unsafe {
            let mut obj = None;
            self.services.GetObject(&BSTR::from(class_name), WBEM_FLAG_RETURN_WBEM_COMPLETE, None, Some(&mut obj), None)?;
            Ok(obj.unwrap())
        }
    }
}

impl WmiClient {
    pub fn connect(namespace: &str) -> Result<Self> {
        unsafe {
            let locator: IWbemLocator = CoCreateInstance(&WbemLocator, None, CLSCTX_INPROC_SERVER)?;
            let services = locator.ConnectServer(
                &BSTR::from(namespace),
                &BSTR::default(),
                &BSTR::default(),
                &BSTR::default(),
                0,
                &BSTR::default(),
                None,
            )?;

            CoSetProxyBlanket(
                &services,
                RPC_C_AUTHN_WINNT,
                RPC_C_AUTHZ_NONE,
                None,
                RPC_C_AUTHN_LEVEL_CALL,
                RPC_C_IMP_LEVEL_IMPERSONATE,
                None,
                EOAC_NONE,
            )?;

            Ok(Self { services })
        }
    }
}

impl Clone for WmiClient {
    fn clone(&self) -> Self {
        Self {
            services: self.services.clone(),
        }
    }
}

pub struct WmiResult {
    enum_obj: IEnumWbemClassObject,
}

impl WmiResult {
    pub fn new(enum_obj: IEnumWbemClassObject) -> Self {
        Self { enum_obj }
    }
}

impl Iterator for WmiResult {
    type Item = Result<IWbemClassObject>;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let mut objects = [None; 1];
            let mut returned = 0;
            let result = self.enum_obj.Next(WBEM_INFINITE, &mut objects, &mut returned);
            if result.is_err() || returned == 0 {
                None
            } else {
                Some(Ok(objects[0].clone().unwrap()))
            }
        }
    }
}

pub fn get_property(obj: &IWbemClassObject, name: &str) -> Result<VARIANT> {
    unsafe {
        let mut value = VARIANT::default();
        obj.Get(&BSTR::from(name), 0, &mut value, None, None)?;
        Ok(value)
    }
}

pub fn get_property_names(obj: &IWbemClassObject) -> Result<Vec<String>> {
    unsafe {
        let mut names = Vec::new();
        let qualifier = BSTR::default();
        let names_ptr = obj.GetNames(&qualifier, WBEM_CONDITION_FLAG_TYPE(0), std::ptr::null())?;
        let count = SafeArrayGetUBound(names_ptr, 1)? - SafeArrayGetLBound(names_ptr, 1)? + 1;
        for i in 0..count {
            let mut bstr = BSTR::default();
            let indices = [i as i32];
            SafeArrayGetElement(names_ptr, indices.as_ptr() as *const _, &mut bstr as *mut _ as *mut _)?;
            let name = bstr.to_string();
            if !name.starts_with("__") {
                names.push(name);
            }
        }
        SafeArrayDestroy(names_ptr)?;
        Ok(names)
    }
}

pub fn variant_to_value(v: &VARIANT) -> Value {
    unsafe {
        let vt = v.Anonymous.Anonymous.vt;
        if (vt.0 & VT_ARRAY.0) != 0 {
            let sa = v.Anonymous.Anonymous.Anonymous.parray;
            if sa.is_null() {
                return Value::Array(vec![]);
            }
            let mut arr = Vec::new();
            let lbound = SafeArrayGetLBound(sa, 1).unwrap_or(0);
            let ubound = SafeArrayGetUBound(sa, 1).unwrap_or(-1);
            for _ in lbound..=ubound {
                // Simplified: WMI arrays can be complex, just putting placeholders for now
                // or we could try to get elements if we know the type.
                arr.push(Value::String("...".to_string()));
            }
            return Value::Array(arr);
        }

        match vt {
            VT_BSTR => Value::String(v.Anonymous.Anonymous.Anonymous.bstrVal.to_string()),
            VT_I4 => Value::Number(v.Anonymous.Anonymous.Anonymous.lVal.into()),
            VT_UI4 => Value::Number(v.Anonymous.Anonymous.Anonymous.ulVal.into()),
            VT_I2 => Value::Number(v.Anonymous.Anonymous.Anonymous.iVal.into()),
            VT_UI2 => Value::Number(v.Anonymous.Anonymous.Anonymous.uiVal.into()),
            VT_BOOL => Value::Bool(v.Anonymous.Anonymous.Anonymous.boolVal.0 != 0),
            VT_NULL => Value::Null,
            VT_EMPTY => Value::String("".to_string()),
            _ => Value::String(format!("{:?}", vt)),
        }
    }
}

pub fn wmi_obj_to_json(obj: &IWbemClassObject) -> Result<Value> {
    let names = get_property_names(obj)?;
    let mut map = Map::new();
    for name in names {
        let val = get_property(obj, &name)?;
        map.insert(name, variant_to_value(&val));
    }
    Ok(Value::Object(map))
}

pub fn get_object_text(obj: &IWbemClassObject) -> Result<String> {
    unsafe {
        let text = obj.GetObjectText(0)?;
        Ok(text.to_string())
    }
}

pub fn variant_to_string(v: &VARIANT) -> String {
    let val = variant_to_value(v);
    match val {
        Value::String(s) => s,
        Value::Null => "".to_string(),
        _ => val.to_string(),
    }
}
