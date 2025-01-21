### omdbapi.com ###  
# API               https://www.omdbapi.com/
# API xml exemple:  tt0412142   1408  House md   https://www.omdbapi.com/?i=tt0412142
# API xml exemple:  tt0186151  10559  Frequency  https://www.omdbapi.com/?i=tt0186151

### Imports ###
# Python Modules #
import os
# HAMA Modules #
import common
from common import Log, DictString, Dict, SaveDict # Direct import of heavily used functions

### Variables ###
OMDB_HTTP_API_URL = "https://www.omdbapi.com/?apikey={api_key}&i="

### Functions ###
def GetMetadata(movie, IMDbid):  # return 200 but not downloaded correctly - IMDB has a single poster, downloading through OMDB xml, prefered by mapping file
  Log.Info("=== OMDb.GetMetadata() ===".ljust(157, '='))
  url       = OMDB_HTTP_API_URL.format(api_key=Prefs['OMDbApiKey']) #'
  OMDb_dict = {}

  if Prefs['OMDbApiKey'] in ('None', '', 'N/A'):  Log.Info("No api key found - Prefs['OMDbApiKey']: '{}'".format(Prefs['OMDbApiKey']));  return OMDb_dict
  
  Log.Info("IMDbid: '%s'" % IMDbid)
  for imdbid_single in IMDbid.split(",") if IMDbid.startswith("tt") else []:
    Log.Info(("--- %s.series ---" % imdbid_single).ljust(157, '-'))
    json = common.LoadFile(filename=imdbid_single+".json", relativeDirectory=os.path.join('OMDb', 'json'), url=url + imdbid_single)
    if json:
      Log.Info("[ ] title: {}"                  .format(SaveDict( Dict(json,'Title')     , OMDb_dict, 'title'                  )))
      Log.Info("[ ] summary: {}"                .format(SaveDict( Dict(json,'Plot')      , OMDb_dict, 'summary'                )))
      Log.Info("[ ] originally_available_at: {}".format(SaveDict( Dict(json,'Released')  , OMDb_dict, 'originally_available_at')))
      Log.Info("[ ] countries: {}"              .format(SaveDict( Dict(json,'Country')   , OMDb_dict, 'countries'              )))
      Log.Info("[ ] directors: {}"              .format(SaveDict( Dict(json,'Director')  , OMDb_dict, 'directors'              )))
      Log.Info("[ ] genres: {}"                 .format(SaveDict( sorted([x.strip() for x in Dict(json,'Genre').split(',')]), OMDb_dict, 'genres')))
      Log.Info("[ ] writers: {}"                .format(SaveDict( Dict(json,'Writer')    , OMDb_dict, 'writers'                )))
      SaveDict( Dict(json,'imdbRating'), OMDb_dict, 'rating')
      if Dict(json,'Metascore').isdigit() and not Dict(OMDb_dict,'rating'):
        SaveDict( float(json['Metascore'])/10, OMDb_dict, 'rating')
      Log.Info("[ ] rating: {}".format(Dict(OMDb_dict,'rating')))
      if SaveDict( Dict(json,'Rated'), OMDb_dict, 'content_rating') in common.Movie_to_Serie_US_rating and not movie and Dict(json,'Type')=="movie":
        Log.Info("[ ] content_rating: {}".format(SaveDict(common.Movie_to_Serie_US_rating[json['Rated']], OMDb_dict, 'content_rating')))
      if Dict(json,'Poster'):  Log.Info("[ ] poster: {}".format(json['Poster'])); SaveDict((os.path.join('OMDb', 'poster', imdbid_single+'.jpg'), common.poster_rank('OMDb', 'posters'), None), OMDb_dict, 'posters', json['Poster'])
      try:     Log.Info("[ ] duration: {}".format(SaveDict( int(Dict(json,'Runtime').replace(' min','')) * 60 * 1000, OMDb_dict, 'duration')))  # Plex save duration in millisecs
      except:  pass

  Log.Info("--- return ---".ljust(157, '-'))
  Log.Info("OMDb_dict: {}".format(DictString(OMDb_dict, 4)))
  return OMDb_dict
