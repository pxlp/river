
typedef void * PixelportApp;
typedef long PixelportEntityId;

extern "C" PixelportApp pixelport_new();
extern "C" bool pixelport_update(PixelportApp app);
extern "C" PixelportEntityId pixelport_get_root(PixelportApp app);
extern "C" PixelportEntityId pixelport_append_entity(PixelportApp app, PixelportEntityId parent_id, char *type_name);
extern "C" void pixelport_set_property(PixelportApp app, PixelportEntityId entity_id, char *property_key, char *expression);
