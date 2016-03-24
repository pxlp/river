
typedef void * PixelportApp;
typedef long PixelportEntityId;

extern "C" PixelportApp pixelport_new();
extern "C" bool pixelport_update(PixelportApp app);
extern "C" PixelportEntityId pixelport_request(PixelportApp app, char *request);
