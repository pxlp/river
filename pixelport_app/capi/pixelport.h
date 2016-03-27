
typedef void * PixelportApp;
typedef long PixelportChannelId;
typedef PixelportChannelId PixelportRequestId;

extern "C" PixelportApp pixelport_new();
extern "C" bool pixelport_update(PixelportApp app);
extern "C" PixelportRequestId pixelport_request(PixelportApp app, char *request);
