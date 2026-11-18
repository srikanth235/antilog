// Viewer-request function on the web app's behavior only (never /api/* or /internal/*).
// A path with no file extension is a screen of the single-page app: serve /index.html so a
// reload or a pasted link works. A real file (an asset) is left alone, and a missing one is
// a real 404 from S3 instead of the app's HTML.
function handler(event) {
  var request = event.request;
  var last = request.uri.split('/').pop();
  if (last.indexOf('.') === -1) {
    request.uri = '/index.html';
  }
  return request;
}
